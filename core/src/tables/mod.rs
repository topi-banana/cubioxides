//! Auto-generated decision-tree tables used by 1.18+ biome climate
//! sampling. The actual data lives in 5 build-script-generated files
//! (one per MC sub-version), which we `include!` from `OUT_DIR`.
//!
//! The tables come from cubiomes' `tables/btree*.h`. Each table is
//! consumed by [`BiomeTree`] (in [`crate::biomenoise::climate`]) to
//! map a 6-dimensional climate point to a biome id.

// The generated tables contain hundreds of thousands of literals,
// many of which would trip individual clippy lints (unreadable
// literal, long literal lacking separators, etc.). The build script
// can't reasonably emit `_` separators for every hex node value, so
// we silence the relevant lints for the included files.
#[allow(
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings,
    clippy::needless_pass_by_value,
    clippy::all
)]
mod btree18 {
    include!(concat!(env!("OUT_DIR"), "/btree18.rs"));
}
#[allow(
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings,
    clippy::needless_pass_by_value,
    clippy::all
)]
mod btree192 {
    include!(concat!(env!("OUT_DIR"), "/btree192.rs"));
}
#[allow(
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings,
    clippy::needless_pass_by_value,
    clippy::all
)]
mod btree19 {
    include!(concat!(env!("OUT_DIR"), "/btree19.rs"));
}
#[allow(
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings,
    clippy::needless_pass_by_value,
    clippy::all
)]
mod btree20 {
    include!(concat!(env!("OUT_DIR"), "/btree20.rs"));
}
#[allow(
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings,
    clippy::needless_pass_by_value,
    clippy::all
)]
mod btree21wd {
    include!(concat!(env!("OUT_DIR"), "/btree21wd.rs"));
}

use crate::biomenoise::climate::BiomeTree;

/// 1.18 / 1.18.2 climate decision tree.
pub const BTREE_18: BiomeTree = BiomeTree {
    steps: btree18::BTREE18_STEPS,
    param: btree18::BTREE18_PARAM,
    nodes: btree18::BTREE18_NODES,
    order: btree18::BTREE18_ORDER,
};

/// 1.19.2 climate decision tree.
pub const BTREE_192: BiomeTree = BiomeTree {
    steps: btree192::BTREE192_STEPS,
    param: btree192::BTREE192_PARAM,
    nodes: btree192::BTREE192_NODES,
    order: btree192::BTREE192_ORDER,
};

/// 1.19.4 ("1.19") climate decision tree.
pub const BTREE_19: BiomeTree = BiomeTree {
    steps: btree19::BTREE19_STEPS,
    param: btree19::BTREE19_PARAM,
    nodes: btree19::BTREE19_NODES,
    order: btree19::BTREE19_ORDER,
};

/// 1.20.6 ("1.20") climate decision tree.
pub const BTREE_20: BiomeTree = BiomeTree {
    steps: btree20::BTREE20_STEPS,
    param: btree20::BTREE20_PARAM,
    nodes: btree20::BTREE20_NODES,
    order: btree20::BTREE20_ORDER,
};

/// 1.21 Winter Drop climate decision tree.
pub const BTREE_21WD: BiomeTree = BiomeTree {
    steps: btree21wd::BTREE21WD_STEPS,
    param: btree21wd::BTREE21WD_PARAM,
    nodes: btree21wd::BTREE21WD_NODES,
    order: btree21wd::BTREE21WD_ORDER,
};
