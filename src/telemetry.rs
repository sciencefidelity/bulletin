use std::time::Duration;

use axum::http::{Request, Response};
use axum::{Router, body::Body};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Span, Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};
use uuid::Uuid;

pub enum Formatter {
    Bunyan,
    Log,
    Otel,
}

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    formatter: &Formatter,
    sink: Sink,
) -> Box<dyn Subscriber + Send + Sync>
where
    Sink: for<'a> fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    match formatter {
        Formatter::Bunyan => Box::new(bunyan_subscriber(filter_layer, name, sink)),
        Formatter::Log => Box::new(log_subscriber(filter_layer)),
        Formatter::Otel => Box::new(otel_subscriber(filter_layer, name)),
    }
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn tracing_layer(router: Router) -> Router {
    router.layer(
        TraceLayer::new_for_http()
            .make_span_with(|_request: &Request<Body>| {
                let request_id = Uuid::new_v4().to_string();
                tracing::info_span!("http-request", %request_id)
            })
            .on_request(|request: &Request<Body>, _span: &Span| {
                tracing::info!("request: {} {}", request.method(), request.uri().path());
            })
            .on_response(
                |response: &Response<Body>, latency: Duration, _span: &Span| {
                    tracing::info!("response: {} {:?}", response.status(), latency);
                },
            )
            .on_failure(
                |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                    tracing::error!("error: {}", error);
                },
            ),
    )
}

fn bunyan_subscriber<Sink>(
    filter_layer: EnvFilter,
    name: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(filter_layer)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn log_subscriber(filter_layer: EnvFilter) -> impl Subscriber + Send + Sync {
    let formatting_layer = fmt::layer();
    Registry::default()
        .with(filter_layer)
        .with(formatting_layer)
}

fn otel_subscriber(filter_layer: EnvFilter, name: String) -> impl Subscriber + Send + Sync {
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();
    let tracer = provider.tracer(name);
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    Registry::default().with(filter_layer).with(telemetry_layer)
}
