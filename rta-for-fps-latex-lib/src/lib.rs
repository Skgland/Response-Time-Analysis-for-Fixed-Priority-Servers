use std::fmt::{Display, Formatter};

use rta_for_fps_lib::window::WindowEnd;
use rta_for_fps_lib::{
    curve::curve_types::CurveType, curve::Curve, window::Demand, window::Window,
};

pub struct TotalDemandCurve {
    steps: Vec<Window<Demand>>,
}

impl Display for TotalDemandCurve {
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

impl TotalDemandCurve {
    pub fn new<C: CurveType<WindowKind = Demand>>(curve: Curve<C>) -> Self {
        TotalDemandCurve {
            steps: curve.into_windows(),
        }
    }
}

pub struct CurveWindows<W> {
    windows: Vec<Window<W>>,
}

impl<W> Display for CurveWindows<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for window in self.windows.iter() {
            let length = match window.length() {
                WindowEnd::Finite(length) => length,
                WindowEnd::Infinite => continue,
            };
            writeln!(
                f,
                "\\fill ({start}.0, 0.0) rectangle ++({length}.0, 1.0);",
                start = window.start.as_unit(),
                length = length.as_unit()
            )?;
        }
        Ok(())
    }
}

impl<W> CurveWindows<W> {
    pub fn new<C: CurveType<WindowKind = W>>(curve: Curve<C>) -> Self {
        CurveWindows {
            windows: curve.into_windows(),
        }
    }
}
