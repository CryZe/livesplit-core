pub enum Lang {
    English,
    German,
}

#[derive(Copy, Clone)]
pub enum Text {
    StartSplit,
}

impl Text {
    pub fn resolve(self, lang: Lang) -> &'static str {
        match lang {
            Lang::English => return resolve_english(self),
            Lang::German => resolve_german(self),
        }
        .unwrap_or_else(|| resolve_english(self))
    }
}

const fn resolve_english(text: Text) -> &'static str {
    match text {
        Text::StartSplit => "Start / Split",
    }
}

const fn resolve_german(text: Text) -> Option<&'static str> {
    None
}
