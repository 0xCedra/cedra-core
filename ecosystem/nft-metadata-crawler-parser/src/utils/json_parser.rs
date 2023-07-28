// Copyright © Aptos Foundation

use crate::{
    get_uri_metadata,
    utils::constants::{MAX_FILE_SIZE_BYTES, MAX_RETRY_TIME_SECONDS},
};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use image::ImageFormat;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::{error, info};

pub struct JSONParser;

impl JSONParser {
    /// Parses JSON from input URI.
    /// Returns the underlying raw image URI, raw animation URI, and JSON.
    pub async fn parse(uri: String) -> anyhow::Result<(Option<String>, Option<String>, Value)> {
        let (mime, size) = get_uri_metadata(uri.clone()).await?;
        if ImageFormat::from_mime_type(mime).is_some() {
            error!(uri = uri, "JSON parser received image URI, skipping");
            return Err(anyhow::anyhow!("JSON parser received image URI, skipping"));
        } else if size > MAX_FILE_SIZE_BYTES {
            error!(uri = uri, "JSON parser received large file, skipping");
            return Err(anyhow::anyhow!("JSON parser received large file, skipping"));
        }

        let op = || {
            async {
                info!("Sending request for token_uri {}", uri);

                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_RETRY_TIME_SECONDS / 3))
                    .build()
                    .context("Failed to build reqwest client")?;

                let response = client
                    .get(&uri)
                    .send()
                    .await
                    .context("Failed to get JSON")?;

                let parsed_json = response
                    .json::<Value>()
                    .await
                    .context("Failed to parse JSON")?;

                let raw_image_uri = parsed_json["image"].as_str().map(|s| s.to_string());
                let raw_animation_uri =
                    parsed_json["animation_url"].as_str().map(|s| s.to_string());

                Ok((raw_image_uri, raw_animation_uri, parsed_json))
            }
            .boxed()
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, op).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(uri = uri, "Exponential backoff timed out, skipping JSON");
                Err(e)
            },
        }
    }
}
