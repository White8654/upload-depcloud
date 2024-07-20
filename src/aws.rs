use rusoto_core::Region;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::task;
use std::env;
use rusoto_core::credential::StaticProvider;
use rusoto_core::HttpClient;
use std::io::Read;
use dotenv::dotenv;

pub struct S3Uploader {
    client: S3Client,
    bucket: String,
}

impl S3Uploader {
    
    pub fn new(bucket: String, region: Region) -> Self {
        dotenv().ok();

        let access_key = env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set");
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY must be set");

        let dispatcher = HttpClient::new().expect("failed to create request dispatcher");
        let provider = StaticProvider::new_minimal(access_key, secret_key);
        let client = S3Client::new_with(dispatcher, provider, region);
        Self { client, bucket }
    }

    pub fn upload_folder<'a>(&'a self, id: String, folder_path: &'a Path) -> BoxFuture<'a, Result<(), Box<dyn Error>>> {
        async move {
            let entries = fs::read_dir(folder_path)?;

            for entry in entries {
                println!("folder: {:?}", folder_path);
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    self.upload_folder(id.clone(), &path).await?;
                } else {
                    self.upload_file(id.clone(), folder_path, &path).await?;
                }
            }
            Ok(())
        }.boxed()
    }

    async fn upload_file(&self, id: String, base_folder: &Path, file_path: &Path) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file_path)?;
        let metadata = file.metadata()?;
        let mut buffer = vec![0; metadata.len() as usize];
        file.read_exact(&mut buffer)?;

        // Create the key by removing the base folder part of the path
        let b = "output";
        let result = format!("{}/{}", b, id);
        let relative_path = file_path.strip_prefix(result)?;
        let key = format!("{}/{}", id, relative_path.display());
        println!("Uploading file: {}", key);

        let put_request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key,
            body: Some(buffer.into()),
            ..Default::default()
        };

        self.client.put_object(put_request).await?;
        Ok(())
    }
}
