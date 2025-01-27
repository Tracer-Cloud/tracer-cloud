#[derive(Clone, Debug)]
pub enum AwsConfig {
    Profile(String),
    RoleArn(String),
    Env,
}
