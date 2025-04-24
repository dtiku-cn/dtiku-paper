pub mod embedding;

use tonic::{Code, Status};

trait AnyhowToStatus {
    fn to_status(self, code: Code) -> Status;
}

impl AnyhowToStatus for anyhow::Error {
    fn to_status(self, code: Code) -> Status {
        Status::new(code, self.to_string())
    }
}
