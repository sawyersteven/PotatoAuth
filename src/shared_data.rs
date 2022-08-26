use actix_web::{web, HttpRequest};

/// Methods to make sharing and extracting a struct through app_data easier
/// Create a sharable T by implementing this trait and setting the type Sharable
/// to the desired type (eg Mutex<Foo>)
/// The type can be extracted from a HttpRequest with T::extract_from(&req), which
/// will return a Data<Shared>, which is effectively an Arc<Shared>.
pub trait Sharable {
    type Shared;

    /// Returns a web::Data<Shared> that can be safely shared between threads
    /// as an app_data::<T>() on a HttpRequest
    fn to_sharable(self) -> web::Data<Self::Shared>;

    /// Extracts the sharable type from a HttpRequest as &web::Data<Shared>
    fn extract_from(req: &HttpRequest) -> &web::Data<Self::Shared>
    where
        Self::Shared: 'static,
    {
        return req.app_data::<web::Data<Self::Shared>>().unwrap();
    }
}
