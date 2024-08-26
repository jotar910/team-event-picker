#[derive(Debug, PartialEq)]
pub enum FindError {
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for FindError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FindAllError {
    Unknown,
}

impl From<mongodb::error::Error> for FindAllError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InsertError {
    Conflict,
    Unknown,
}

impl From<mongodb::error::Error> for InsertError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

impl From<bson::ser::Error> for InsertError {
    fn from(value: bson::ser::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UpdateError {
    Conflict,
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for UpdateError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

impl From<bson::ser::Error> for UpdateError {
    fn from(value: bson::ser::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DeleteError {
    NotFound,
    Unknown,
}

impl From<mongodb::error::Error> for DeleteError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CountError {
    Unknown,
}

impl From<mongodb::error::Error> for CountError {
    fn from(value: mongodb::error::Error) -> Self {
        log::error!("occurred an error in mongodb: {}", value);
        match value.kind {
            _ => Self::Unknown,
        }
    }
}
