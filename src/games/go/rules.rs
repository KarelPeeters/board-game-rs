/// The specific Go rules used.
/// See [KataGo's supported rules](https://lightvector.github.io/KataGo/rules.html) for an overview of the variants.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    pub allow_multi_stone_suicide: bool,
    // implicit: game end after two passes, always allowed to pass
    // implicit: positional SuperKo: don't allow moves that repeat the previous stone arrangement
    // implicit: scoring: area scoring
}

impl Rules {
    /// Tromp-Taylor rules, see https://tromp.github.io/go.html.
    pub fn tromp_taylor() -> Self {
        Rules {
            allow_multi_stone_suicide: true,
        }
    }

    /// Rules used by the [Computer Go Server](http://www.yss-aya.com/cgos/).
    /// The same as Tromp-Taylor except suicide is not allowed.
    pub fn cgos() -> Self {
        Rules {
            allow_multi_stone_suicide: false,
        }
    }
}
