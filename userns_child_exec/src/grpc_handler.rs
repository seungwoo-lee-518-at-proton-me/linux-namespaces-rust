use std::path::Path;

use tonic::{Request, Response, Status};
use userns::userns_mapper_server::UsernsMapper;
use userns::MapRequest;

pub mod userns {
    tonic::include_proto!("userns");
}

#[derive(Default)]
pub struct UsernsMapperImpl {}

#[tonic::async_trait]
impl UsernsMapper for UsernsMapperImpl {
    /// handles ping request
    async fn ping(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }
    /// handles uid / gid mapping request
    async fn map(&self, request: Request<MapRequest>) -> Result<Response<()>, Status> {
        if request.get_ref().pid == 0 {
            return Err(Status::new(tonic::Code::InvalidArgument, "invalid id_outside_ns"));
        }
        let pid_directory = Path::new("/proc").join(Path::new(&request.get_ref().pid.to_string()));
        info!("check directory is exists: {}", pid_directory.to_str().unwrap());
        if !pid_directory.exists() {
            warn!("directory does not found: {}", pid_directory.to_str().unwrap());
            return Err(Status::new(tonic::Code::NotFound, "process does not found"));
        }
        // handles UID / GID Mapping
        match request.get_ref().r#type() {
            userns::map_request::Type::Gid => {
                let setgroups_path = pid_directory.join("setgroups");
                info!("echo deny >> {}", setgroups_path.to_str().unwrap());
                if let Err(err) = tokio::fs::write(setgroups_path.clone(), b"deny").await {
                    warn!("echo deny >> {} failed: {}", setgroups_path.to_str().unwrap(), err);
                    return Err(Status::new(tonic::Code::Internal, "setgroups failed"));
                }
                let write_val = format!("{} {} {}\n", 
                                        request.get_ref().id_inside_ns.to_string(),
                                        request.get_ref().id_outside_ns.to_string(),
                                        request.get_ref().length.to_string()
                                        );
                let gid_map_path = pid_directory.join("gid_map");
                info!("echo {} >> {}", write_val.as_str(), gid_map_path.to_str().unwrap());
                if let Err(err) = tokio::fs::write(gid_map_path.clone(), write_val.clone()).await {
                    warn!("echo {} >> {} failed: {}", write_val.as_str(), gid_map_path.to_str().unwrap(), err);
                    return Err(Status::new(tonic::Code::Internal, "gid_map write failed"));
                }
                return Ok(Response::new(()))
            },
            userns::map_request::Type::Uid => {
                let write_val = format!("{} {} {}\n", 
                                        request.get_ref().id_inside_ns.to_string(),
                                        request.get_ref().id_outside_ns.to_string(),
                                        request.get_ref().length.to_string()
                                        );
                let gid_map_path = pid_directory.join("uid_map");
                info!("echo {} >> {}", write_val.as_str(), gid_map_path.to_str().unwrap());
                if let Err(err) = tokio::fs::write(gid_map_path.clone(), write_val.clone()).await {
                    warn!("echo {} >> {} failed: {}", write_val.as_str(), gid_map_path.to_str().unwrap(), err);
                    return Err(Status::new(tonic::Code::Internal, "uid_map write failed"));
                }
                return Ok(Response::new(()))
            }
        };
    }
}


