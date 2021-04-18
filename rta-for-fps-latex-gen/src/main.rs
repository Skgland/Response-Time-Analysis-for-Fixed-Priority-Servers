use rta_for_fps_latex_lib::DemandCurveDataPoints;
use rta_for_fps_lib::curve::AggregateExt;
use rta_for_fps_lib::iterators::{CurveIterator, ReclassifyIterator};
use rta_for_fps_lib::task::curve_types::TaskDemand;
use rta_for_fps_lib::task::Task;
use rta_for_fps_lib::time::TimeUnit;
use rta_for_fps_lib::window::Window;

fn main() -> std::io::Result<()> {
    let t_1 = Task::new(1, 5, 0);
    let t_2 = Task::new(2, 8, 0);

    let condition = |window: &Window<_>| window.end <= TimeUnit::from(50);

    let t1_curve = t_1.into_iter().take_while(condition);
    let t2_curve = t_2.into_iter().take_while(condition);

    let t1 = t1_curve.clone().collect_curve();
    let t2 = t2_curve.clone().collect_curve();

    std::fs::write(
        "latex/data/t1.csv",
        DemandCurveDataPoints::new(t1).to_string(),
    )?;

    std::fs::write(
        "latex/data/t2.csv",
        DemandCurveDataPoints::new(t2).to_string(),
    )?;

    let aggregate = vec![t1_curve, t2_curve]
        .into_iter()
        .aggregate::<ReclassifyIterator<_, TaskDemand>>()
        .collect_curve();

    std::fs::write(
        "latex/data/t1andt2.csv",
        DemandCurveDataPoints::new(aggregate).to_string(),
    )?;

    Ok(())
}
