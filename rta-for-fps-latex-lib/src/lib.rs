use std::fmt::{Display, Formatter};

use rta_for_fps_lib::window::WindowEnd;
use rta_for_fps_lib::{
    curve::curve_types::CurveType, curve::Curve, window::Demand, window::Window,
};

pub struct DemandCurveDataPoints {
    steps: Vec<Window<Demand>>,
}

impl Display for DemandCurveDataPoints {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "x,y")?;
        let mut summed_demand = 0;

        for window in &self.steps {
            let window_start = window.start.as_unit();
            writeln!(f, "{x},{y}", x = window_start, y = summed_demand)?;

            match window.length() {
                WindowEnd::Finite(length) => {
                    let length = length.as_unit();
                    let window_end = window_start + length;
                    summed_demand += length;
                    writeln!(f, "{x},{y}", x = window_end, y = summed_demand)?;
                }
                WindowEnd::Infinite => {}
            }
        }
        Ok(())
    }
}

impl DemandCurveDataPoints {
    pub fn new<C: CurveType<WindowKind = Demand>>(curve: Curve<C>) -> Self {
        DemandCurveDataPoints {
            steps: curve.into_windows(),
        }
    }
}
