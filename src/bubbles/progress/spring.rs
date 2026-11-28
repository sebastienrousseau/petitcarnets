//! A damped spring, for the progress bar's animation: Ryan Juckett's
//! closed form, as `charmbracelet/harmonica` v0.2.0 ports it.

/// The smallest difference from 1.0, Go's `epsilon` there.
const EPSILON: f64 = f64::EPSILON;

/// A spring's precomputed coefficients for one time step: harmonica's
/// `Spring`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Spring {
    pos_pos: f64,
    pos_vel: f64,
    vel_pos: f64,
    vel_vel: f64,
}

impl Spring {
    /// A spring stepping `delta` seconds at `frequency` (radians a
    /// second) with `damping` (1.0 is critical): harmonica's `NewSpring`.
    pub(super) fn new(delta: f64, frequency: f64, damping: f64) -> Self {
        let (w, z) = (frequency.max(0.0), damping.max(0.0));
        if w < EPSILON {
            return Self {
                pos_pos: 1.0,
                pos_vel: 0.0,
                vel_pos: 0.0,
                vel_vel: 1.0,
            };
        }
        if z > 1.0 + EPSILON {
            over_damped(delta, w, z)
        } else if z < 1.0 - EPSILON {
            under_damped(delta, w, z)
        } else {
            let exp = (-w * delta).exp();
            let time_exp = delta * exp;
            let time_exp_freq = time_exp * w;
            Self {
                pos_pos: time_exp_freq + exp,
                pos_vel: time_exp,
                vel_pos: -w * time_exp_freq,
                vel_vel: -time_exp_freq + exp,
            }
        }
    }

    /// The position and velocity one step on, towards `target`.
    pub(super) fn update(&self, pos: f64, vel: f64, target: f64) -> (f64, f64) {
        let old = pos - target;
        (
            old * self.pos_pos + vel * self.pos_vel + target,
            old * self.vel_pos + vel * self.vel_vel,
        )
    }
}

/// The coefficients of an over damped spring.
fn over_damped(delta: f64, w: f64, z: f64) -> Spring {
    let za = -w * z;
    let zb = w * (z * z - 1.0).sqrt();
    let (z1, z2) = (za - zb, za + zb);
    let (e1, e2) = ((z1 * delta).exp(), (z2 * delta).exp());
    let inv = 1.0 / (2.0 * zb);
    let (e1_over, e2_over) = (e1 * inv, e2 * inv);
    let (z1e1_over, z2e2_over) = (z1 * e1_over, z2 * e2_over);
    Spring {
        pos_pos: e1_over * z2 - z2e2_over + e2,
        pos_vel: -e1_over + e2_over,
        vel_pos: (z1e1_over - z2e2_over + e2) * z2,
        vel_vel: -z1e1_over + z2e2_over,
    }
}

/// The coefficients of an under damped spring.
fn under_damped(delta: f64, w: f64, z: f64) -> Spring {
    let omega_zeta = w * z;
    let alpha = w * (1.0 - z * z).sqrt();
    let exp = (-omega_zeta * delta).exp();
    let (cos, sin) = ((alpha * delta).cos(), (alpha * delta).sin());
    let inv_alpha = 1.0 / alpha;
    let (exp_sin, exp_cos) = (exp * sin, exp * cos);
    let exp_omega_zeta_sin_over_alpha = exp * omega_zeta * sin * inv_alpha;
    Spring {
        pos_pos: exp_cos + exp_omega_zeta_sin_over_alpha,
        pos_vel: exp_sin * inv_alpha,
        vel_pos: -exp_sin * alpha - omega_zeta * exp_omega_zeta_sin_over_alpha,
        vel_vel: exp_cos - exp_omega_zeta_sin_over_alpha,
    }
}
