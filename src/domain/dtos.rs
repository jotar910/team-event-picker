#[derive(Debug, PartialEq)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
}

impl<T> ListResponse<T> {
    pub fn new(data: Vec<T>) -> ListResponse<T> {
        ListResponse { data }
    }
}
