use muxy_app_core::dictionary::{Dictionary, PATH};
use muxy_client::{Client, ClientError};
use muxy_protocol::{ErrorCode, FilesAction, FilesReply, FilesRequest, ProjectId, ServerPath};
use muxy_ui::tr;

/// The first dictionary among `projects`, such as a worktree and then the
/// project it came from. A project without one has an empty dictionary.
pub(super) fn load(client: &Client, projects: &[ProjectId]) -> Result<Dictionary, String> {
    for &project in projects {
        let reply = client.files(FilesRequest {
            project,
            action: FilesAction::Read(ServerPath(PATH.as_bytes().to_vec())),
        });
        match reply {
            Ok(FilesReply::Content(file)) => return Dictionary::parse(&file.content),
            Ok(_) => return Err(tr!("Unexpected dictionary file response").to_string()),
            Err(ClientError::Server(reply)) if reply.code == ErrorCode::BadPath => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(Dictionary::default())
}
