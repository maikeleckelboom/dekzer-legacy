mod projections;

// Current projection state and retention live here; older replay helpers stay out of the
// module center so the crate API stays focused on the maintained projection flow.
pub(crate) use projections::{
    invalidate_projection_domain, reseed_current_projection_state, reseed_projection_domains,
    sweep_projection_retention,
};
