//! The Harris–Wilson / BLV allocation dynamic (ADR 0013 Decision 2).
//!
//! Integrated as a gradient flow in `x = ln W`, which is the
//! coordinate Ellam et al. state the flow in and the reason the
//! `δ/κ` floor works: as `W → 0` the log-space drift tends to
//! `ε(D_j + δ) > 0`, so a shrinking centre is pushed back up instead
//! of crossing zero. Integrating in `W` would need a positivity
//! clamp, and a negative `W_j` makes `W_j^α` NaN.
//!
//! The solve runs on **shares**: `Σ_j W_j = K` with `κ = (ΣO_i +
//! δM)/K`. That normalisation is what makes `ε` a pure rate and the
//! stability bound dimensionless; on absolute head-counts every
//! published `ε` is inapplicable and the runtime assertion fires on
//! every run.

/// Stability margin on `ε · dt · max_j (D_j + δ)`. The fixed point is
/// marginally stable at 2 and chaos onsets at 2.57 (research 0016's
/// algebra on Wilson's difference equation with May 1976 Table I);
/// 0.5 is the margin that dossier recommends asserting at runtime,
/// and it is a margin rather than a threshold partly because the
/// log-coordinate map is of the exponential family, whose bifurcation
/// constants are not the logistic map's and are not imported here.
pub const STABILITY_MARGIN: f64 = 0.5;

#[derive(Clone, Debug)]
pub struct HwParams {
    /// Returns to scale. Calibrated, not declared (ADR 0013 D1).
    pub alpha: f64,
    /// Distance decay, per hour.
    pub beta: f64,
    /// Response rate; numerical, paired with `dt`.
    pub epsilon: f64,
    pub dt: f64,
    /// Minimum-size term. Must be > 0: at δ = 0 the Gibbs measure is
    /// unnormalisable and dead zones become absorbing.
    pub delta: f64,
    /// Total settled mass the shares are expressed in.
    pub k_total: f64,
    pub max_iters: u32,
    /// Convergence on `max_j |D_j − κW_j + δ|` in share units.
    pub tol: f64,
}

#[derive(Clone, Debug)]
pub struct HwSolution {
    /// Converged sizes, in the share units of `k_total`.
    pub w: Vec<f64>,
    pub d: Vec<f64>,
    pub kappa: f64,
    pub iters: u32,
    pub converged: bool,
    /// Final value of `max_j |D_j − κW_j + δ|`.
    pub residual: f64,
    /// Largest `ε · dt · max_j (D_j + δ)` seen over the whole run —
    /// the quantity ADR 0013 gate 4 re-reads from disk, recorded
    /// because a per-step check cannot be verified after the fact.
    pub max_step_multiplier: f64,
}

/// Solve one component. `o` is the origin mass per vertex and `kernel`
/// is the row-major `exp(−β c_ij)` matrix, both of length `n` / `n²`.
///
/// `kernel` is passed in rather than built here because it depends
/// only on `β` and the cost field, so a sweep over `α` reuses it.
pub fn solve(o: &[f64], kernel: &[f64], p: &HwParams) -> HwSolution {
    let n = o.len();
    assert_eq!(kernel.len(), n * n, "kernel must be n×n row-major");
    assert!(p.delta > 0.0, "delta must be > 0 (ADR 0013 Decision 2)");

    let m = n as f64;
    let o_sum: f64 = o.iter().sum();
    let kappa = (o_sum + p.delta * m) / p.k_total;

    // Start from an equal share of the budget: the engine's own
    // symmetric state, so any dispersion in the result came from the
    // terrain rather than from the initial condition.
    let mut x: Vec<f64> = vec![(p.k_total / m).ln(); n];
    let mut w = vec![0.0f64; n];
    let mut wa = vec![0.0f64; n];
    let mut d = vec![0.0f64; n];
    let mut acc = vec![0.0f64; n];

    let mut iters = 0;
    let mut converged = false;
    let mut residual = f64::INFINITY;
    let mut max_step_multiplier = 0.0f64;

    while iters < p.max_iters {
        for j in 0..n {
            w[j] = x[j].exp();
            wa[j] = w[j].powf(p.alpha);
        }
        // A_i = 1 / Σ_k W_k^α exp(−β c_ik); then accumulate A_i O_i
        // into the destination column weights.
        acc.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..n {
            let row = &kernel[i * n..(i + 1) * n];
            let mut s = 0.0;
            for j in 0..n {
                s += row[j] * wa[j];
            }
            if !s.is_finite() || s <= 0.0 {
                // Origin reaches nothing (or the row underflowed):
                // contributes no flow rather than dividing by zero and
                // seeding NaN through every downstream gate.
                continue;
            }
            let aio = o[i] / s;
            for j in 0..n {
                acc[j] += aio * row[j];
            }
        }
        let mut res = 0.0f64;
        let mut step_mult = 0.0f64;
        for j in 0..n {
            d[j] = wa[j] * acc[j];
            let drift = d[j] - kappa * w[j] + p.delta;
            res = res.max(drift.abs());
            step_mult = step_mult.max(p.epsilon * p.dt * (d[j] + p.delta).abs());
        }
        max_step_multiplier = max_step_multiplier.max(step_mult);
        residual = res;
        iters += 1;
        if res <= p.tol {
            converged = true;
            break;
        }
        for j in 0..n {
            x[j] += p.epsilon * p.dt * (d[j] - kappa * w[j] + p.delta);
        }
    }

    HwSolution {
        w,
        d,
        kappa,
        iters,
        converged,
        residual,
        max_step_multiplier,
    }
}

/// Find the interior equilibrium by damped successive substitution:
/// `W ← (1−λ)W + λ (D(W) + δ)/κ`.
///
/// **This is a solver, not the model.** The model is the BLV flow in
/// `solve`; this routine only locates its fixed point. That is
/// legitimate precisely because ADR 0013 gate 4 states the residual on
/// the *equilibrium condition* `max_j |D_j − κW_j + δ|`, which is
/// solver-independent — any method that drives it below tolerance has
/// found the same equilibrium.
///
/// The reason it is needed: time-stepping the log flow has a slowest
/// mode of rate `ε·dt·δ` at starved sites (see `solve`), and at a `δ`
/// small enough that floors hold only a minor share of the budget,
/// that rate implies of order a million iterations. The damped
/// substitution converges at `|1 − λ|` per step, with no `δ`
/// dependence at all. Transients — epochs, shocks, anything where the
/// path matters rather than the endpoint — still use `solve`.
pub fn solve_equilibrium(o: &[f64], kernel: &[f64], p: &HwParams, lambda: f64) -> HwSolution {
    let n = o.len();
    assert_eq!(kernel.len(), n * n, "kernel must be n×n row-major");
    assert!(p.delta > 0.0, "delta must be > 0 (ADR 0013 Decision 2)");
    assert!(
        lambda > 0.0 && lambda <= 1.0,
        "relaxation must be in (0, 1]"
    );

    let m = n as f64;
    let o_sum: f64 = o.iter().sum();
    let kappa = (o_sum + p.delta * m) / p.k_total;

    let mut w: Vec<f64> = vec![p.k_total / m; n];
    let mut wa = vec![0.0f64; n];
    let mut d = vec![0.0f64; n];
    let mut acc = vec![0.0f64; n];

    let mut iters = 0;
    let mut converged = false;
    let mut residual = f64::INFINITY;

    while iters < p.max_iters {
        for j in 0..n {
            wa[j] = w[j].powf(p.alpha);
        }
        acc.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..n {
            let row = &kernel[i * n..(i + 1) * n];
            let mut s = 0.0;
            for j in 0..n {
                s += row[j] * wa[j];
            }
            if !s.is_finite() || s <= 0.0 {
                continue;
            }
            let aio = o[i] / s;
            for j in 0..n {
                acc[j] += aio * row[j];
            }
        }
        let mut res = 0.0f64;
        for j in 0..n {
            d[j] = wa[j] * acc[j];
            res = res.max((d[j] - kappa * w[j] + p.delta).abs());
        }
        residual = res;
        iters += 1;
        if res <= p.tol {
            converged = true;
            break;
        }
        for j in 0..n {
            let target = (d[j] + p.delta) / kappa;
            w[j] = (1.0 - lambda) * w[j] + lambda * target;
            // The floor is structural, not a clamp: `target` is never
            // below δ/κ because `d[j] >= 0`. Guard only against a
            // denormal creeping in, which would make `W^α` lose the
            // interior branch.
            if !w[j].is_finite() || w[j] <= 0.0 {
                w[j] = p.delta / kappa;
            }
        }
    }

    HwSolution {
        w,
        d,
        kappa,
        iters,
        converged,
        residual,
        // Not a time-stepped run: the stability bound does not apply,
        // and reporting a fabricated multiplier would be worse than
        // reporting none.
        max_step_multiplier: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> HwParams {
        HwParams {
            alpha: 1.05,
            beta: 1.0,
            epsilon: 0.05,
            dt: 1.0,
            delta: 1e-4,
            k_total: 1.0,
            max_iters: 200_000,
            tol: 1e-12,
        }
    }

    /// Two symmetric sites with equal origin mass and a symmetric cost
    /// must end equal — this is Osawa's equal-sized-centres result in
    /// miniature, and it is the check that the engine adds no
    /// dispersion of its own.
    #[test]
    fn symmetric_sites_stay_equal_and_conserve_mass() {
        let n = 2;
        let c = 3.0f64;
        let p = params();
        let k = vec![1.0, (-p.beta * c).exp(), (-p.beta * c).exp(), 1.0];
        let o = vec![0.5, 0.5];
        let s = solve(&o, &k, &p);
        assert!(s.converged, "did not converge: residual {}", s.residual);
        assert!(
            (s.w[0] - s.w[1]).abs() < 1e-12,
            "symmetry broken: {:?}",
            s.w
        );
        // Σκ W = ΣO + δM, the ADR 0013 gate-1 identity.
        let lhs = s.kappa * (s.w[0] + s.w[1]);
        let rhs = o.iter().sum::<f64>() + p.delta * n as f64;
        assert!(
            ((lhs - rhs) / rhs).abs() < 1e-9,
            "mass not conserved: {lhs} vs {rhs}"
        );
    }

    /// The two solvers must land on the same equilibrium — that is
    /// the whole justification for using the fast one to find it.
    #[test]
    fn flow_and_fixed_point_agree_on_the_equilibrium() {
        let p = HwParams {
            epsilon: 0.4,
            dt: 1.0,
            delta: 0.01,
            tol: 1e-11,
            ..params()
        };
        let c = 2.0f64;
        let k = vec![1.0, (-p.beta * c).exp(), (-p.beta * c).exp(), 1.0];
        let o = vec![0.7, 0.3];
        let a = solve(&o, &k, &p);
        let b = solve_equilibrium(&o, &k, &p, 0.5);
        assert!(a.converged && b.converged);
        for j in 0..2 {
            assert!(
                (a.w[j] - b.w[j]).abs() / a.w[j] < 1e-6,
                "solvers disagree at {j}: {} vs {}",
                a.w[j],
                b.w[j]
            );
        }
        assert!(
            b.iters < a.iters,
            "fixed point should be the faster route: {} vs {}",
            b.iters,
            a.iters
        );
    }

    /// With `δ > 0` a site that receives no demand at all still holds
    /// the floor `δ/κ` rather than collapsing to zero.
    ///
    /// **The parameters here are not the defaults, and the reason is a
    /// real property of the dynamic.** Linearising the log-coordinate
    /// map at a starved site gives multiplier `1 − ε·dt·δ`, so the
    /// approach to the floor has rate `ε·dt·δ` — the slowest mode in
    /// the system, and independent of everything else. At `δ = 1e-4`
    /// and `ε·dt = 0.05` that is `5e-6` per step, which needs millions
    /// of iterations to settle and blows any sane cap. This test uses
    /// `δ = 0.01` and `ε·dt = 0.4` (still inside the 0.5 stability
    /// margin, since `max_j D_j ≈ 1`) to make the floor reachable.
    /// The iteration budget of a real run has to be set against
    /// `ε·dt·δ`, not against the fast modes.
    #[test]
    fn isolated_site_holds_the_delta_over_kappa_floor() {
        let p = HwParams {
            epsilon: 0.4,
            dt: 1.0,
            delta: 0.01,
            tol: 1e-11,
            ..params()
        };
        // Two sites, mutually unreachable: kernel is the identity, so
        // each is its own only destination.
        let k = vec![1.0, 0.0, 0.0, 1.0];
        let o = vec![1.0, 0.0];
        let s = solve(&o, &k, &p);
        assert!(s.converged, "did not converge: residual {}", s.residual);
        let floor = p.delta / s.kappa;
        assert!(
            s.w[1] > 0.0 && (s.w[1] - floor).abs() / floor < 1e-6,
            "starved site should sit at δ/κ = {floor}, got {}",
            s.w[1]
        );
    }
}
