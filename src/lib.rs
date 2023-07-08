use std::{
    collections::HashMap,
    ops::AddAssign,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tracing::{span, Subscriber};
use tracing_subscriber::{registry::LookupSpan, Layer};

pub struct DurationLayer {
    inner: Arc<Mutex<Inner>>,
}

struct StartedAt(Instant);

struct Inner {
    duration: HashMap<&'static str, Duration>,
}

impl<S> Layer<S> for DurationLayer
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        _attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();

        span.extensions_mut().insert(StartedAt(Instant::now()));
    }

    fn on_close(&self, id: span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(&id).unwrap();

        let name = span.metadata().name();
        let started_at = span.extensions().get::<StartedAt>().unwrap().0;

        span.extensions_mut().remove::<StartedAt>();

        self.inner
            .lock()
            .unwrap()
            .duration
            .entry(name)
            .or_default()
            .add_assign(started_at.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
