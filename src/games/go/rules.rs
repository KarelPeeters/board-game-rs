use std::fmt::{Debug, Formatter};

/// The specific Go rules used.
/// See [KataGo's supported rules](https://lightvector.github.io/KataGo/rules.html) for an overview of the variants.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    pub allow_multi_stone_suicide: bool,
    // implicit: repeating tiles not allowed
    // implicit: game end after two passes, always allowed to pass
    // implicit: scoring: area scoring
}

impl Rules {
    const NAMED_RULES: &'static [(&'static str, Rules)] = &[("TT", Rules::tromp_taylor()), ("CGOS", Rules::cgos())];

    /// Tromp-Taylor rules, see <https://tromp.github.io/go.html>.
    pub const fn tromp_taylor() -> Self {
        Rules {
            allow_multi_stone_suicide: true,
        }
    }

    /// Rules used by the [Computer Go Server](http://www.yss-aya.com/cgos/).
    /// The same as Tromp-Taylor except suicide is not allowed.
    pub const fn cgos() -> Self {
        Rules {
            allow_multi_stone_suicide: false,
        }
    }

    pub fn needs_history(&self) -> bool {
        true
    }

    pub fn allow_repeating_tiles(&self) -> bool {
        false
    }
}

impl Debug for Rules {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = Rules::NAMED_RULES.iter().find(|(_, r)| r == self).map(|(n, _)| n);
        if let Some(name) = name {
            write!(f, "Rules({:?})", name)
        } else {
            f.debug_struct("Rules")
                .field("allow_multi_stone_suicide", &self.allow_multi_stone_suicide)
                .finish()
        }
    }
}
