pub use speedrun_com_api as api;

pub use api::Client;

use crate::{
    timing::formatter::{Regular, TimeFormatter},
    Run, TimeSpan,
};
use api::{common::Id, leaderboards::CountryCode, platforms, regions, runs::Players, Game};
use futures_util::{
    future, pin_mut,
    stream::{StreamExt, TryStreamExt},
};
use hashbrown::{HashMap, HashSet};
use ordered_float::OrderedFloat;
use snafu::{OptionExt, ResultExt, Snafu};
use std::{future::Future, sync::Arc};

#[derive(Debug, Snafu)]
pub enum LeaderboardError {
    /// The name of the game provided does not match any game on speedrun.com.
    GameNotFound,
    /// An error occurred while searching for the game.
    GameSearch { source: api::Error },
    /// The name of the category provided does not match any category on
    /// speedrun.com.
    CategoryNotFound,
    /// An error occurred while searching for the category.
    CategorySearch { source: api::Error },
    /// Failed downloading the runs of the leaderboard.
    Runs { source: api::Error },
    /// Despite having requested the players to be embedded, the server did not
    /// embed the players.
    PlayersNotEmbedded,
    /// Failed downloading the variables for the game.
    Variables { source: api::Error },
    /// A run contains a variable that doesn't exist.
    VariableDoesntExist,
    /// A run contains a variable with a value that doesn't exist.
    VariableValueDoesntExist,
    /// Failed downloading the platforms.
    Platforms { source: api::Error },
    /// A run contains a platform that doesn't exist.
    PlatformDoesntExist,
    /// Failed downloading the regions.
    Regions { source: api::Error },
    /// A run contains a region that doesn't exist.
    RegionDoesntExist,
}

pub struct Leaderboard {
    runs: Vec<RunInfo>,
    hide_obsolete: bool,
}

struct RunInfo {
    link: Arc<str>,
    players: Arc<[Player]>,
    time: Arc<str>,
    video: Option<Arc<str>>,
    comment: Option<Arc<str>>,
    platform: Arc<str>,
    region: Option<Arc<str>>,
    variables: Arc<[[Arc<str>; 2]]>,
    splits: Option<Arc<str>>,
    seconds: f64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Player {
    is_user: bool,
    country_code: Option<CountryCode>,
    name: Box<str>,
}

struct Variable {
    name: Arc<str>,
    values: HashMap<Id, Arc<str>>,
}

impl Leaderboard {
    pub fn for_run<'client>(
        client: &'client Client,
        run: &Run,
    ) -> impl Future<Output = Result<Self, LeaderboardError>> + 'client {
        let game = run.game_name().to_owned();
        let category = run.category_name().to_owned();
        async move { Self::new(client, &game, &category).await }
    }

    pub async fn new(
        client: &Client,
        game: &str,
        category: &str,
    ) -> Result<Self, LeaderboardError> {
        let game = {
            let search = Game::search(client, game);
            pin_mut!(search);
            loop {
                let potential_game = search
                    .next()
                    .await
                    .context(GameNotFound)?
                    .context(GameSearch)?;

                if &*potential_game.names.international == game {
                    break potential_game;
                }
            }
        };

        // TODO: try_join to error out early
        let (platforms, regions, categories, variables) = future::join4(
            platforms::all(client, None)
                .and_then(|p| async move { Ok((p.id, p.name.into())) })
                .try_collect(),
            regions::all(client, None)
                .and_then(|p| async move { Ok((p.id, p.name.into())) })
                .try_collect(),
            game.categories(client),
            game.variables(client),
        )
        .await;

        let platforms: HashMap<_, Arc<str>> = platforms.context(Platforms)?;
        let regions: HashMap<_, Arc<str>> = regions.context(Regions)?;

        // TODO: Do we want to store any of this in the leaderboard struct?
        // Depends on whether we'll want to reuse some of this data for when the
        // game or category changes.
        let categories = categories.context(CategorySearch)?;
        let category = categories
            .iter()
            .find(|c| &*c.name == category)
            .context(CategoryNotFound)?;

        let variables = variables.context(Variables)?;
        let variables: HashMap<_, _> = variables
            .into_iter()
            .map(|variable| {
                (
                    variable.id,
                    Variable {
                        name: variable.name.into(),
                        values: variable
                            .values
                            .values
                            .into_iter()
                            .map(|(id, value)| (id, value.label.into()))
                            .collect(),
                    },
                )
            })
            .collect();

        // TODO: Respect formatting based on the game info
        let formatter = Regular::new();

        let mut runs: Vec<_> = api::runs::get(
            client,
            Some(&category.id),
            Some(500),
            Some(api::runs::RunStatus::Verified),
            api::runs::Embeds::PLAYERS,
        )
        .map_err(|source| LeaderboardError::Runs { source })
        .and_then(|r| {
            async {
                let splits = r.splits_id().map(|s| s.into());

                let players = match r.players {
                    Players::Embedded { data } => data
                        .into_iter()
                        .map(|player| {
                            let (is_user, country_code, name) = match dbg!(player) {
                                api::runs::Player::User(user) => (
                                    true,
                                    user.location.and_then(|l| l.country.emoji()),
                                    user.names.international,
                                ),
                                api::runs::Player::Guest(guest) => (false, None, guest.name),
                            };

                            dbg!(Player {
                                is_user,
                                country_code,
                                name,
                            })
                        })
                        .collect::<Vec<_>>()
                        .into(),
                    _ => return Err(LeaderboardError::PlayersNotEmbedded),
                };

                let time = formatter
                    .format(TimeSpan::from_seconds(r.times.primary_t))
                    .to_string()
                    .into();

                let platform = platforms
                    .get(&r.system.platform)
                    .context(PlatformDoesntExist)?
                    .clone();

                let region = if let Some(region) = &r.system.region {
                    Some(regions.get(region).context(RegionDoesntExist)?.clone())
                } else {
                    None
                };

                let variables = r
                    .values
                    .into_iter()
                    .map(|(key, value)| {
                        let variable = variables.get(&key).context(VariableDoesntExist)?;
                        let name = variable.name.clone();
                        let value = variable
                            .values
                            .get(&value)
                            .context(VariableValueDoesntExist)?
                            .clone();

                        Ok([name, value])
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into();

                let videos = r.videos;
                let video = catch! {
                    videos?.links?.into_iter().next()?.uri.into()
                };

                let comment = r.comment.map(Into::into);
                let link = r.weblink.into();

                Ok(RunInfo {
                    link,
                    players,
                    time,
                    video,
                    comment,
                    platform,
                    region,
                    variables,
                    splits,
                    seconds: r.times.primary_t,
                })
            }
        })
        .try_collect()
        .await?;

        // TODO: Maybe we want to reduce the amount of data we actually want to
        // store. I don't think we want to store whole api::Runs, there's way
        // too much in there.

        // TODO: Do we need anything else to be respected when sorting? Maybe
        // unstable sort is also questionable, because it may result in
        // different sortings every time. Not sure if it matters.
        runs.sort_unstable_by_key(|r| OrderedFloat(r.seconds));

        Ok(Self {
            runs,
            hide_obsolete: true,
        })
    }

    pub fn state(&self) -> State {
        let mut seen_before = HashSet::new();
        let mut last_seconds = -0.0;
        let mut rank = 0;
        let mut last_rank = 0;

        let runs = self
            .runs
            .iter()
            .filter_map(|r| {
                let seen_before = !seen_before.insert(r.players.clone());

                let this_rank = if seen_before {
                    if self.hide_obsolete {
                        return None;
                    }
                    None
                } else {
                    rank += 1;
                    if r.seconds > last_seconds {
                        last_seconds = r.seconds;
                        last_rank = rank;
                        Some(rank)
                    } else {
                        Some(last_rank)
                    }
                };

                Some(RunState {
                    rank: this_rank,
                    link: r.link.clone(),
                    players: r.players.clone(),
                    time: r.time.clone(),
                    video: r.video.clone(),
                    comment: r.comment.clone(),
                    platform: r.platform.clone(),
                    region: r.region.clone(),
                    variables: r.variables.clone(),
                    splits: r.splits.clone(),
                })
            })
            .collect();

        State { runs }
    }

    pub fn set_hide_obsolete(&mut self, hide_obsolete: bool) {
        self.hide_obsolete = hide_obsolete;
    }
}

#[derive(Debug)]
pub struct State {
    runs: Vec<RunState>,
}

#[derive(Debug)]
pub struct RunState {
    rank: Option<u32>,
    link: Arc<str>,
    players: Arc<[Player]>,
    time: Arc<str>,
    video: Option<Arc<str>>,
    comment: Option<Arc<str>>,
    platform: Arc<str>,
    region: Option<Arc<str>>,
    variables: Arc<[[Arc<str>; 2]]>,
    splits: Option<Arc<str>>,
}

#[cfg(test)]
mod tests {
    use super::{Client, Leaderboard};

    #[tokio::test]
    async fn test() {
        let client = Client::new();
        let mut leaderboard =
            Leaderboard::new(&client, "The Legend of Zelda: The Wind Waker", "Any%")
                .await
                .unwrap();

        let state = leaderboard.state();

        dbg!(state);

        // leaderboard.set_hide_obsolete(false);

        // let state = leaderboard.state();

        // dbg!(state);

        panic!();
    }
}
