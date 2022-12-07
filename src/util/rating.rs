use crate::wdl::WDL;

/// WDL doesn't have to be normalized yet.
pub fn elo_from_wdl(wdl: WDL<f32>) -> f32 {
    let score = (wdl.value() / wdl.sum() + 1.0) / 2.0;
    let elo = -400.0 * (1.0 / score - 1.0).log10();

    // fix annoying negative zero case
    elo + 0.0
}
