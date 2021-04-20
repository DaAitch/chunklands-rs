pub type Result<T> = std::result::Result<T, GameError>;

#[derive(Debug)]
pub enum GameError {
    VulkanError(String),
}
