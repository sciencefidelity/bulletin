use std::io::IsTerminal;
use std::{net::SocketAddr, time::Duration};

use axum::http::{Request, Response};
use axum::{Router, body::Body, extract::ConnectInfo};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use tower_http::trace::TraceLayer;
use tracing::{Span, Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use uuid::Uuid;

pub enum Formatter {
    Bunyan,
    Log,
    Otel,
    Otlp,
    Stackdriver,
}

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    formatter: &Formatter,
    sink: Sink,
) -> Box<dyn Subscriber + Send + Sync>
where
    Sink: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    match formatter {
        Formatter::Bunyan => Box::new(bunyan_subscriber(filter_layer, name, sink)),
        Formatter::Log => Box::new(log_subscriber(filter_layer)),
        Formatter::Otel => Box::new(otel_subscriber(filter_layer, name)),
        Formatter::Otlp => Box::new(otlp_subscriber(filter_layer, name)),
        Formatter::Stackdriver => Box::new(stackdriver_subscriber(filter_layer)),
    }
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn tracing_layer(router: Router) -> Router {
    router.layer(
        TraceLayer::new_for_http()
            .make_span_with(trace_layer_make_span_with)
            .on_request(trace_layer_on_request)
            .on_response(trace_layer_on_response),
    )
}

fn trace_layer_make_span_with(request: &Request<Body>) -> Span {
    let request_id = Uuid::new_v4().to_string();
    let source = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(
            || tracing::field::display(String::from("<unknown>")),
            |connect_info| tracing::field::display(connect_info.ip().to_string()),
        );

    tracing::info_span!("request",
        %request_id,
        uri = %request.uri(),
        method = %request.method(),
        %source,
        status = tracing::field::Empty,
        latency = tracing::field::Empty,
    )
}

fn trace_layer_on_request(_request: &Request<Body>, _span: &Span) {
    tracing::trace!("Got request");
}

fn trace_layer_on_response(response: &Response<Body>, latency: Duration, span: &Span) {
    span.record(
        "latency",
        // tracing::field::display(format!("{}μs", latency.as_micros())),
        tracing::field::display(format!("{}ms", latency.as_millis())),
    );
    span.record("status", tracing::field::display(response.status()));
    tracing::trace!("Responded");
}

fn bunyan_subscriber<Sink>(
    filter_layer: EnvFilter,
    name: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(filter_layer)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn log_subscriber(filter_layer: EnvFilter) -> impl Subscriber + Send + Sync {
    let formatting_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(std::io::stderr)
        .pretty();
    Registry::default()
        .with(filter_layer)
        .with(tracing_error::ErrorLayer::default())
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

fn otlp_subscriber(filter_layer: EnvFilter, name: String) -> impl Subscriber + Send + Sync {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()
        .expect("building otel subscriber");

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();

    let tracer = provider.tracer(name);
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // let json_layer = tracing_subscriber::fmt::Layer::new()
    //     .with_current_span(false)
    //     .with_span_list(false)
    //     .with_opentelemetry_ids(true);

    Registry::default().with(filter_layer).with(telemetry_layer)
    // .with(json_layer)
}

fn stackdriver_subscriber(filter_layer: EnvFilter) -> impl Subscriber + Send + Sync {
    let stackdriver_layer = tracing_stackdriver::layer();
    Registry::default()
        .with(filter_layer)
        // .with(tracing_error::ErrorLayer::default())
        .with(stackdriver_layer)
}
