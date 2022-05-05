#[macro_use]
extern crate serde;

use std::env;

use futures::TryStreamExt;
use pulsar::{
    Authentication, Consumer, DeserializeMessage, Payload, Pulsar, SubType, TokioExecutor,
};
use tracing::error;
use tracing::info;

#[derive(Serialize, Deserialize)]
struct TestData {
    data: String,
}

impl DeserializeMessage for TestData {
    type Output = Result<TestData, serde_json::Error>;

    fn deserialize_message(payload: &Payload) -> Self::Output {
        serde_json::from_slice(&payload.data)
    }
}

#[tokio::main]
async fn main() -> Result<(), pulsar::Error> {
    tracing_subscriber::fmt::init();
    info!("init");
    let addr = env::var("PULSAR_ADDRESS")
        .ok()
        .unwrap_or_else(|| "pulsar://127.0.0.1:6650".to_string());
    let topic = env::var("PULSAR_TOPIC")
        .ok()
        .unwrap_or_else(|| "non-persistent://public/default/test1".to_string());
        // .unwrap_or_else(|| "persistent://public/default/test1".to_string());

    let mut builder = Pulsar::builder(addr, TokioExecutor);
    info!("builder");
    if let Ok(token) = env::var("PULSAR_TOKEN") {
        let authentication = Authentication {
            name: "token".to_string(),
            data: token.into_bytes(),
        };

        builder = builder.with_auth(authentication);
    }
    info!("builder");
    let pulsar: Pulsar<_> = builder.build().await?;
    info!("builder await");
    let mut consumer: Consumer<TestData, _> = pulsar
        .consumer()
        .with_topic(topic)
        .with_consumer_name("test_consumer")
        .with_subscription_type(SubType::Exclusive)
        .with_subscription("test_subscription")
        .build()
        .await?;

    let mut counter = 0usize;
    while let Some(msg) = consumer.try_next().await? {
        consumer.ack(&msg).await?;
        info!("metadata: {:?}", msg.metadata());
        info!("id: {:?}", msg.message_id());
        let data = match msg.deserialize() {
            Ok(data) => data,
            Err(e) => {
                error!("could not deserialize message: {:?}", e);
                break;
            }
        };

        if data.data.as_str() != "data" {
            error!("Unexpected payload: {}", &data.data);
            break;
        }
        counter += 1;
        info!("got {} messages", counter);
    }

    Ok(())
}