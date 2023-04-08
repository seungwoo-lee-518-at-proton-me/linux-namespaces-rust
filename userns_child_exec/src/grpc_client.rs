pub mod userns {
    tonic::include_proto!("userns");
}

use std::error::Error;

use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use userns::userns_mapper_client::UsernsMapperClient;
use userns::MapRequest;

pub struct UsernsClient {
    client: UsernsMapperClient<tonic::transport::Channel>,
    rt: tokio::runtime::Runtime
}

impl UsernsClient {
    /// Create New Client from Unix Socket ("/tmp/userns.sock")
    pub fn connect() -> Result<Self, tonic::transport::Error> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let client = rt.block_on(async move {
            match Endpoint::try_from("http://any.url").
                unwrap().
                connect_with_connector(tower::service_fn(|_: Uri| {
                    UnixStream::connect("/tmp/userns.sock")
                })).
                await {
                    Ok(channel) => Ok(channel),
                    Err(err) => {
                        return Err(err)
                    }
                }
        });
        match client {
            Ok(client) => {
                // Create Client
                let userns_mapper_client = UsernsMapperClient::new(client);
                Ok(Self { client: userns_mapper_client, rt })
            },
            Err(err) => {
                Err(err)
            }
        }
    }

    /// Send ping Request to Server
    pub fn ping(&mut self) -> Result<(), Box<dyn Error>> {
        match self.rt.block_on(self.client.ping(())) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string().into())
        }
    }

    pub fn map_gid(&mut self, pid: nix::unistd::Pid, uid: nix::unistd::Uid) -> Result<(), Box<dyn Error>> {
        let map_request = tonic::Request::new(MapRequest{
            r#type: userns::map_request::Type::Gid as i32,
            id_inside_ns: 0,
            id_outside_ns: uid.as_raw() as u32,
            length: 1,
            pid: pid.as_raw() as u32
        });
        match self.rt.block_on(self.client.map(map_request)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string().into())
        }
    }

    pub fn map_uid(&mut self, pid: nix::unistd::Pid, uid: nix::unistd::Uid) -> Result<(), Box<dyn Error>> {
        let map_request = tonic::Request::new(MapRequest{
            r#type: userns::map_request::Type::Uid as i32,
            id_inside_ns: 0,
            id_outside_ns: uid.as_raw() as u32,
            length: 1,
            pid: pid.as_raw() as u32
        });
        match self.rt.block_on(self.client.map(map_request)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string().into())
        }
    }
}
