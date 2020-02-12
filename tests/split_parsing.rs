mod run_files;

mod parse {
    use crate::run_files;
    use livesplit_core::run::parser::{
        composite, flitter, llanfair, portal2_live_timer, source_live_timer, splits_io,
        splitterino, splitterz, time_split_tracker, urn, worstrun, wsplit, TimerKind,
    };
    #[cfg(not(miri))]
    use livesplit_core::run::parser::{livesplit, llanfair2, llanfair_gered};
    #[cfg(not(miri))]
    use livesplit_core::{analysis::total_playtime, Run, TimeSpan};
    use std::io::Cursor;

    fn file(data: &[u8]) -> Cursor<&[u8]> {
        Cursor::new(data)
    }

    #[cfg(not(miri))]
    fn livesplit(data: &[u8]) -> Run {
        livesplit::parse(file(data), None).unwrap()
    }

    #[cfg(not(miri))]
    fn parse_llanfair_gered(data: &[u8]) {
        llanfair_gered::parse(file(data)).unwrap();
    }

    #[cfg(not(miri))]
    fn parse_llanfair2(data: &[u8]) {
        llanfair2::parse(file(data)).unwrap();
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_fuzz_crash() {
        livesplit::parse(file(run_files::LIVESPLIT_FUZZ_CRASH), None).unwrap_err();
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_fuzz_crash_utf8() {
        livesplit::parse(file(run_files::LIVESPLIT_FUZZ_CRASH_UTF8), None).unwrap_err();
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_1_0() {
        livesplit(run_files::LIVESPLIT_1_0);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_1_4() {
        livesplit(run_files::LIVESPLIT_1_4);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_1_5() {
        livesplit(run_files::LIVESPLIT_1_5);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_1_6() {
        livesplit(run_files::LIVESPLIT_1_6);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_1_6_gametime() {
        livesplit(run_files::LIVESPLIT_1_6_GAMETIME);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_celeste() {
        livesplit(run_files::CELESTE);
    }

    #[cfg(not(miri))]
    #[test]
    fn livesplit_attempt_ended_bug() {
        let run = livesplit(run_files::LIVESPLIT_ATTEMPT_ENDED_BUG);
        let playtime = total_playtime::calculate(run);
        assert!(playtime >= TimeSpan::zero());
    }

    #[test]
    fn llanfair() {
        llanfair::parse(file(run_files::LLANFAIR)).unwrap();
    }

    #[test]
    fn zeroed_out_doesnt_parse_as_llanfair() {
        llanfair::parse(Cursor::new(&mut [0u8; 64][..])).unwrap_err();
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair_gered_doesnt_parse_as_livesplit() {
        livesplit::parse(file(run_files::LLANFAIR_GERED), None).unwrap_err();
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair_gered() {
        parse_llanfair_gered(run_files::LLANFAIR_GERED);
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair_gered_with_refs() {
        parse_llanfair_gered(run_files::LLANFAIR_GERED_WITH_REFS);
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair_gered_icons() {
        parse_llanfair_gered(run_files::LLANFAIR_GERED_ICONS);
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair2() {
        parse_llanfair2(run_files::LLANFAIR2)
    }

    #[cfg(not(miri))]
    #[test]
    fn llanfair2_empty() {
        parse_llanfair2(run_files::LLANFAIR2_EMPTY)
    }

    #[test]
    fn time_split_tracker() {
        let run = time_split_tracker::parse(file(run_files::TIME_SPLIT_TRACKER), None).unwrap();
        assert_eq!(
            run.custom_comparisons(),
            [
                "Personal Best",
                "Time",
                "Custom Comparison",
                "Race",
                "Race2"
            ]
        );
    }

    #[test]
    fn time_split_tracker_without_attempt_count() {
        time_split_tracker::parse(
            file(run_files::TIME_SPLIT_TRACKER_WITHOUT_ATTEMPT_COUNT),
            None,
        )
        .unwrap();
    }

    #[test]
    fn splitterz() {
        splitterz::parse(file(run_files::SPLITTERZ), false).unwrap();
    }

    #[test]
    fn wsplit() {
        wsplit::parse(file(run_files::WSPLIT), false).unwrap();
    }

    #[test]
    fn splitterino() {
        splitterino::parse(file(run_files::SPLITTERINO)).unwrap();
    }

    #[test]
    fn urn() {
        urn::parse(file(run_files::URN)).unwrap();
    }

    #[test]
    fn flitter() {
        flitter::parse(file(run_files::FLITTER)).unwrap();
    }

    #[test]
    fn flitter_small() {
        flitter::parse(file(run_files::FLITTER_SMALL)).unwrap();
    }

    #[test]
    fn source_live_timer() {
        source_live_timer::parse(file(run_files::SOURCE_LIVE_TIMER)).unwrap();
    }

    #[test]
    fn source_live_timer2() {
        source_live_timer::parse(file(run_files::SOURCE_LIVE_TIMER2)).unwrap();
    }

    #[test]
    fn portal2_live_timer() {
        portal2_live_timer::parse(file(run_files::PORTAL2_LIVE_TIMER1)).unwrap();
    }

    #[test]
    fn portal2_live_timer2() {
        portal2_live_timer::parse(file(run_files::PORTAL2_LIVE_TIMER2)).unwrap();
    }

    #[test]
    fn worstrun() {
        worstrun::parse(file(run_files::WORSTRUN)).unwrap();
    }

    #[test]
    fn splits_io() {
        splits_io::parse(file(run_files::GENERIC_SPLITS_IO)).unwrap();
    }

    #[test]
    fn splits_io_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::GENERIC_SPLITS_IO), None, false).unwrap();
        assert!(if let TimerKind::Generic(_) = run.kind {
            true
        } else {
            false
        });
    }

    #[test]
    fn portal2_live_timer_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::PORTAL2_LIVE_TIMER1), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Portal2LiveTimer);
    }

    #[test]
    fn worstrun_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::WORSTRUN), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Worstrun);
    }

    #[test]
    fn splitterino_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::SPLITTERINO), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Splitterino);
    }

    #[test]
    fn urn_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::URN), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Urn);
    }

    #[test]
    fn source_live_time_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::SOURCE_LIVE_TIMER), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::SourceLiveTimer);
    }

    #[test]
    fn flitter_prefers_parsing_as_itself() {
        let run = composite::parse(file(run_files::FLITTER), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Flitter);

        let run = composite::parse(file(run_files::FLITTER_SMALL), None, false).unwrap();
        assert_eq!(run.kind, TimerKind::Flitter);
    }
}
