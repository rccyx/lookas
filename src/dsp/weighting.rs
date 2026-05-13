#[inline]
#[must_use]
pub fn a_weighting(hz: f32) -> f32 {
    const P1_SQ: f32 = 20.6_f32 * 20.6_f32;
    const P2_SQ: f32 = 107.7_f32 * 107.7_f32;
    const P3_SQ: f32 = 737.9_f32 * 737.9_f32;
    const P4_SQ: f32 = 12_194.0_f32 * 12_194.0_f32;
    const NORM: f32 = 1.258_925_4;

    let f = hz.max(10.0);
    let f2 = f * f;
    let f4 = f2 * f2;

    let num = P4_SQ * f4;
    let den = (f2 + P1_SQ)
        * ((f2 + P2_SQ) * (f2 + P3_SQ)).sqrt()
        * (f2 + P4_SQ);

    (num / den) * NORM
}
