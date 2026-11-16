//! The worker built without the type checker (`worker-core` alone).

use monty_proto::{
    FrameReader, PROTOCOL_VERSION, pb,
    worker::{HandleOutcome, Worker, dispatch_frame},
    write_frame,
};
use monty_types::MONTY_VERSION;

/// Dispatches one request and returns its single reply event.
fn reply(worker: &mut Worker, kind: pb::client_request::Kind) -> pb::worker_event::Kind {
    let mut request = Vec::new();
    write_frame(
        &mut request,
        &pb::ClientRequest {
            kind: Some(kind),
            trace_parent: None,
        },
    )
    .expect("framing a request never fails");
    let (bytes, outcome) = dispatch_frame(worker, &request);
    assert_eq!(outcome, HandleOutcome::Continue);
    let mut reader = FrameReader::new(bytes.as_slice());
    let event = reader
        .read::<pb::WorkerEvent>()
        .expect("the reply decodes")
        .expect("a turn always ends with one event");
    event.kind.expect("event has a kind")
}

fn configure(type_check: bool) -> pb::client_request::Kind {
    pb::client_request::Kind::Configure(pb::Configure {
        script_name: "main.py".to_owned(),
        type_check,
        monty_version: MONTY_VERSION.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        ..Default::default()
    })
}

#[cfg(not(feature = "type-checking"))]
#[test]
fn type_checked_sessions_are_refused_and_unchecked_ones_run() {
    let mut worker = Worker::default();
    match reply(&mut worker, configure(true)) {
        pb::worker_event::Kind::SessionRefused(refused) => {
            assert!(refused.message.contains("type checking"), "{}", refused.message);
        }
        other => panic!("expected SessionRefused, got {other:?}"),
    }
    assert!(matches!(
        reply(&mut worker, configure(false)),
        pb::worker_event::Kind::Ok(_)
    ));
    let fed = reply(
        &mut worker,
        pb::client_request::Kind::Feed(pb::Feed {
            code: "1 + 1".to_owned(),
            cwd: "/".to_owned(),
            ..Default::default()
        }),
    );
    assert!(matches!(fed, pb::worker_event::Kind::Complete(_)), "{fed:?}");
}
