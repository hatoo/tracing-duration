use std::{
    collections::HashMap,
    ops::AddAssign,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tracing::{span, Subscriber};
use tracing_subscriber::{registry::LookupSpan, Layer};

pub struct DurationLayer {
    inner: Arc<Mutex<DurationData>>,
}

#[derive(Clone)]
pub struct DurationLayerContoller {
    inner: Arc<Mutex<DurationData>>,
}

struct StartedAt(Instant);

#[derive(Debug, Clone, Default)]
pub struct DurationRecord {
    pub duration: Duration,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct DurationData {
    pub start: Instant,
    pub duration: HashMap<&'static str, DurationRecord>,
}

impl DurationData {
    // I don't want to impl `Default` because I feel calling Instant::now() in a default() is not good.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            duration: HashMap::new(),
        }
    }
}

impl DurationLayerContoller {
    pub fn current(&self) -> DurationData {
        self.inner.lock().unwrap().clone()
    }

    pub fn reset(&self) -> DurationData {
        let mut res = DurationData::new();
        std::mem::swap(&mut res, &mut self.inner.lock().unwrap());
        res
    }
}

impl DurationLayer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DurationData::new())),
        }
    }

    pub fn controller(&self) -> DurationLayerContoller {
        DurationLayerContoller {
            inner: self.inner.clone(),
        }
    }
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

        let mut lock = self.inner.lock().unwrap();
        let record = lock.duration.entry(name).or_default();
        record.duration.add_assign(started_at.elapsed());
        record.count += 1;
    }
}

#[cfg(test)]
mod tests {
    use tracing::info_span;
    use tracing_subscriber::prelude::*;

    use super::*;

    #[test]
    fn it_works() {
        let layer = DurationLayer::new();
        let controller = layer.controller();

        tracing_subscriber::registry::Registry::default()
            .with(layer)
            .init();

        info_span!("test").in_scope(|| {
            std::thread::sleep(Duration::from_millis(100));
        });

        let data = controller.current();

        assert_eq!(
            &["test"],
            data.duration.into_keys().collect::<Vec<_>>().as_slice()
        );
    }
}
