pub mod domain;
pub mod errors;
pub mod propensity_loader;
pub mod property_loader;
pub mod settings;

// use  serde::de::DeserializeOwned;
// use crate::loader::errors::LoaderError;

// pub trait DataSource<D>
// where
//     D: DeserializedOwned,
// {
//     type Path;
//     type Iter: Iterator<Item = D>;
//
//     fn size(&self) -> usize;
//     fn into_iter(self) -> Self::Iter;
// }
//
// pub trait DataLoader<S, D>
// where
//     S: DataSource<D>,
// {
//     type Error;
//     type QualityMeasure: Default;
//     type DataStore:
//
//     fn run(path: S::Path) -> Result<(), Self::Error> {
//
//     }
//
//     fn post_process_quality(&self, quality: Self::QualityMeasure) -> Result<(), Self::Error> { Ok(()) }
// }
