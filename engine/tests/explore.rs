mod common;
use common::*;

#[test]
#[ignore]
fn explore_points() {
    let mut e = engine("game", r#"{"loading":0}"#, 2);
    e.key(2, true);
    for t in 0..400 { e.frame(); if t % 40 == 0 { let s = state(&e); println!("t={t} pts={} mv={} dead={} lives={}", s["points"], s["movement"], s["dead"], s["lives"]); } }
}
