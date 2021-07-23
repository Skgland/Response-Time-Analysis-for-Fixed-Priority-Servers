use rta_for_fps_latex_lib::{CurveWindows, TotalDemandCurve};
use rta_for_fps_lib::curve::{AggregateExt, Curve};
use rta_for_fps_lib::iterators::curve::CurveSplitIterator;
use rta_for_fps_lib::iterators::{CurveIterator, ReclassifyIterator};
use rta_for_fps_lib::server::{
    ActualServerExecution, AggregatedServerDemand, ConstrainedServerDemand, Server, ServerKind,
    UnconstrainedServerExecution,
};
use rta_for_fps_lib::system::System;
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

    std::fs::write("latex/data/t1.csv", TotalDemandCurve::new(t1).to_string())?;

    std::fs::write("latex/data/t2.csv", TotalDemandCurve::new(t2).to_string())?;

    let aggregate = vec![t1_curve, t2_curve]
        .into_iter()
        .aggregate::<ReclassifyIterator<_, TaskDemand>>()
        .collect_curve();

    std::fs::write(
        "latex/data/t1andt2.csv",
        TotalDemandCurve::new(aggregate).to_string(),
    )?;

    let external_tasks = &[
        Task::new(3, 18, 0),
        Task::new(5, 24, 5),
        Task::new(5, 24, 12),
    ];

    let external_load = Server::new(external_tasks, 16.into(), 24.into(), ServerKind::Deferrable);

    let server_tasks = &[
        Task::new(1, 24, 2),
        Task::new(1, 24, 5),
        Task::new(2, 24, 10),
    ];

    let server = Server::new(server_tasks, 2.into(), 10.into(), ServerKind::Deferrable);

    let servers = &[external_load, server];

    let system = System::new(servers);

    let limit = TimeUnit::from(24);

    let hp_load = System::aggregated_higher_priority_demand_curve_iter(std::iter::once(
        servers[0].constraint_demand_curve_iter(),
    ));
    let hp_load_first24 =
        CurveSplitIterator::new(hp_load.clone(), limit).take_while(|window| window.end <= limit);

    std::fs::write(
        "latex/data/external_load.tex",
        CurveWindows::new(unsafe {
            Curve::<AggregatedServerDemand>::from_windows_unchecked(hp_load_first24.collect())
        })
        .to_string(),
    )?;

    let execution = CurveSplitIterator::new(
        system.original_unconstrained_server_execution_curve_iter(1),
        limit,
    )
    .take_while(|window| window.end <= limit);

    std::fs::write(
        "latex/data/unconstrained_execution.tex",
        CurveWindows::new(unsafe {
            Curve::<UnconstrainedServerExecution>::from_windows_unchecked(execution.collect())
        })
        .to_string(),
    )?;

    let server_demand = servers[1].constraint_demand_curve_iter();

    let demand =
        CurveSplitIterator::new(server_demand, limit).take_while(|window| window.end <= limit);

    std::fs::write(
        "latex/data/server_demand.tex",
        CurveWindows::new(unsafe {
            Curve::<ConstrainedServerDemand>::from_windows_unchecked(demand.collect())
        })
        .to_string(),
    )?;

    let actual_execution =
        CurveSplitIterator::new(system.original_actual_execution_curve_iter(1), limit)
            .take_while(|window| window.end <= limit);

    std::fs::write(
        "latex/data/actual_execution.tex",
        CurveWindows::new(unsafe {
            Curve::<ActualServerExecution>::from_windows_unchecked(actual_execution.collect())
        })
        .to_string(),
    )?;

    Ok(())
}
