#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SearchLanguage {
    #[default]
    English,
    SimplifiedChinese,
}

pub(crate) struct SearchStrings {
    pub find: &'static str,
    pub replace: &'static str,
    pub flags: [&'static str; 3],
    pub buttons: [(&'static str, &'static str); 5],
}

impl SearchLanguage {
    pub(crate) fn from_tag(tag: &str) -> Self {
        let mut subtags = tag.split(['-', '_']);
        if subtags.next().is_some_and(|tag| tag.eq_ignore_ascii_case("zh"))
            && subtags.next().is_some_and(|tag| {
                ["Hans", "CN", "SG"].iter().any(|subtag| tag.eq_ignore_ascii_case(subtag))
            })
        {
            Self::SimplifiedChinese
        } else {
            Self::English
        }
    }

    pub(crate) const fn strings(self) -> SearchStrings {
        match self {
            Self::English => SearchStrings {
                find: "Find...",
                replace: "Replace",
                flags: ["Match case", "Whole words", "Regex"],
                buttons: [
                    ("Previous", "Find previous (Shift+F3)"),
                    ("Next", "Find next (F3)"),
                    ("Close", "Close (Esc)"),
                    ("Replace", "Replace next (Alt+R)"),
                    ("Replace all", "Replace all (Alt+A)"),
                ],
            },
            Self::SimplifiedChinese => SearchStrings {
                find: "查找...",
                replace: "替换",
                flags: ["区分大小写", "全字匹配", "使用正则表达式"],
                buttons: [
                    ("上一个", "上一个匹配项 (Shift+F3)"),
                    ("下一个", "下一个匹配项 (F3)"),
                    ("关闭", "关闭 (Esc)"),
                    ("替换", "替换 (Alt+R)"),
                    ("全部替换", "全部替换 (Alt+A)"),
                ],
            },
        }
    }

    pub(crate) fn status(self, count: usize, current: Option<usize>, error: Option<&str>) -> String {
        match (self, error, current, count) {
            (Self::English, Some(error), _, _) => format!("Error: {error}"),
            (Self::SimplifiedChinese, Some(error), _, _) => format!("错误：{error}"),
            (Self::English, _, Some(current), _) => format!("{} of {count}", current + 1),
            (Self::SimplifiedChinese, _, Some(current), _) => format!("第{}项，共{count}项", current + 1),
            (Self::English, _, _, 0) => "No matches found".into(),
            (Self::English, _, _, 1) => "1 match".into(),
            (Self::English, _, _, _) => format!("{count} matches"),
            (Self::SimplifiedChinese, _, _, 0) => "无结果".into(),
            (Self::SimplifiedChinese, _, _, _) => format!("{count}个匹配项"),
        }
    }
}
