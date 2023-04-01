use core::fmt::Debug;

pub struct RecoveryFile<T>
where
    T: Recovery,
{
    state: T,
    clusters: Vec<u32>,
    data: Vec<u8>,
}

impl<T> Debug for RecoveryFile<T>
where
    T: Recovery + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecoveryFile")
            .field("state", &self.state)
            .field("clusters", &self.clusters)
            .finish()
    }
}

impl<T> RecoveryFile<T>
where
    T: Recovery,
{
    pub(crate) fn new(state: T, first_cluster: u32, data: Vec<u8>) -> Self {
        Self {
            state,
            clusters: vec![first_cluster],
            data,
        }
    }

    pub(crate) fn cluster_belongs_to_file(&mut self, cluster: u32, data: &[u8]) -> ClusterBelongs {
        let result = self.state.cluster_belongs_to_file(data);
        if result == ClusterBelongs::ToFile || result == ClusterBelongs::IsEndOfFile {
            self.clusters.push(cluster);
            let mut data = data.to_vec();
            self.data.append(&mut data);
        }
        result
    }

    pub fn get_data(self) -> (Vec<u8>, T) {
        (self.data, self.state)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClusterBelongs {
    NotToFile,
    IsEndOfFile,
    ToFile,
}

pub trait Recovery {
    fn cluster_belongs_to_file(&mut self, cluster: &[u8]) -> ClusterBelongs;
}

pub trait RecoveryFactory {
    type State: Recovery;
    fn is_start_of_file(&mut self, cluster: &[u8]) -> Option<Self::State>;
}
