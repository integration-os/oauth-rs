use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use fake::{Fake, Faker};
use integrationos_domain::{
    access_key_data::AccessKeyData, access_key_prefix::AccessKeyPrefix,
    connection_model_definition::ConnectionModelDefinition, encrypted_data::PASSWORD_LENGTH,
    environment::Environment, event_access::EventAccess, event_type::EventType, prefix::IdPrefix,
    AccessKey, Id, Store,
};
use mongodb::{Client as MongoClient, Database};
use oauth_refresh::Configuration;
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::{header::HeaderMap, Client};
use std::collections::HashMap;
use uuid::Uuid;

pub struct TestApp {
    client: Client,
    address: String,
    configuration: Configuration,
    mongo: Database,
}

static IV: Lazy<[u8; 16]> = Lazy::new(|| rand::thread_rng().gen::<[u8; 16]>());
pub static EPOCH: Lazy<DateTime<Utc>> = Lazy::new(|| {
    TimeZone::from_utc_datetime(
        &Utc,
        &NaiveDateTime::from_timestamp_opt(0, 0).expect("Failed to create timestamp"),
    )
});
pub static ID: Lazy<Id> = Lazy::new(|| Id::now(IdPrefix::ConnectionModelDefinition));
pub static EVENT_ACCESS_PASSWORD: Lazy<[u8; PASSWORD_LENGTH]> = Lazy::new(|| {
    "32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS"
        .as_bytes()
        .try_into()
        .expect("Failed to convert password to array")
});

impl TestApp {
    #[cfg(test)]
    pub async fn get<T: Into<String>>(&self, path: T) -> reqwest::Response {
        let path = path.into();
        self.client
            .get(format!("{}/v1/{}", self.address, path))
            .send()
            .await
            .expect("Failed to execute request")
    }

    #[cfg(test)]
    pub async fn post<T: Into<String>, B: Into<reqwest::Body>>(
        &self,
        path: T,
        body: B,
        headers: Option<HeaderMap>,
    ) -> reqwest::Response {
        let path = path.into();
        let headers = headers.unwrap_or_default();
        self.client
            .post(format!("{}/v1/{}", self.address, path))
            .headers(headers)
            .body(body.into())
            .send()
            .await
            .expect("Failed to execute request")
    }

    #[cfg(test)]
    pub async fn spawn(config: HashMap<&str, &str>) -> Self {
        use std::collections::hash_map::RandomState;

        use oauth_refresh::Application;

        let url = "mongodb://127.0.0.1:27017/?directConnection=true";
        let uuid = Uuid::new_v4().to_string();

        let configuration = Configuration::from(
            HashMap::<&str, &str, RandomState>::from_iter([
                ("HOST", "localhost"),
                ("PORT", "0"),
                ("CONTROL_DATABASE_URL", url),
                ("EVENT_DATABASE_URL", url),
                ("CONTEXT_DATABASE_URL", url),
                ("UDM_DATABASE_URL", url),
                ("EVENT_DATABASE_NAME", uuid.as_str()),
                ("CONTEXT_DATABASE_NAME", uuid.as_str()),
                ("CONTROL_DATABASE_NAME", uuid.as_str()),
                ("UDM_DATABASE_NAME", uuid.as_str()),
                ("SECRETS_SERVICE_BASE_URL", "http://localhost:1080/"),
                ("SECRETS_SERVICE_GET_PATH", "v1/secrets/get/"),
                ("SECRETS_SERVICE_CREATE_PATH", "v1/secrets/create/"),
                ("REFRESH_BEFORE_IN_MINUTES", "10"),
                ("SLEEP_TIMER_IN_SECONDS", "20"),
                ("ENVIRONMENT", "development"),
            ])
            .into_iter()
            .chain(config.into_iter())
            .collect::<HashMap<_, _>>(),
        );

        let application = Application::start(&configuration)
            .await
            .expect("Failed to start app");
        let address = format!("http://localhost:{}", application.port());
        tokio::spawn(application.spawn());

        let client = MongoClient::with_uri_str(url)
            .await
            .expect("Failed to create database client")
            .database(uuid.as_str());

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        Self {
            client: Client::new(),
            address,
            configuration,
            mongo: client,
        }
    }

    pub async fn insert_connection_definition(&self) {
        let mut stripe_model_config: ConnectionModelDefinition = Faker.fake();

        stripe_model_config.id = *ID;

        let _ = self
            .mongo
            .collection("connection_model_definition")
            .insert_one(stripe_model_config, None)
            .await
            .expect("Failed to insert into database");
    }

    pub async fn insert_event_access(&self) -> EventAccess {
        let mut event_access: EventAccess = Faker.fake();

        let access_key = self.access_key_encoded();

        event_access.access_key = access_key;
        event_access.paths = Default::default();
        event_access.throughput = Default::default();
        event_access.record_metadata = Default::default();

        let _ = self
            .mongo
            .collection::<EventAccess>(Store::EventAccess.to_string().as_str())
            .insert_one(event_access.clone(), None)
            .await
            .expect("Failed to insert into database");

        event_access
    }

    fn access_key_encoded(&self) -> String {
        let access_key = AccessKey {
            prefix: AccessKeyPrefix {
                environment: Environment::Test,
                event_type: EventType::SecretKey,
                version: 1,
            },
            data: AccessKeyData {
                id: Id::now(IdPrefix::EventAccess).to_string(),
                namespace: "namespace".to_string(),
                event_type: "event_type".to_string(),
                group: "group".to_string(),
                event_path: "event_path".to_string(),
                event_object_id_path: None,
                timestamp_path: None,
                parent_access_key: None,
            },
        };

        let access_key_encoded = access_key
            .encode(&EVENT_ACCESS_PASSWORD, &IV)
            .expect("Failed to encode access key");

        access_key_encoded.to_string()
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn mongo(&self) -> &Database {
        &self.mongo
    }
}
