use std::ops;

use serde::{Deserialize, Serialize};
use sled::{Db, IVec, Tree};

use crate::error::{Error, Result};

pub(crate) const PROJECTION_POSITIONS_TREE: &str = "thalo:projection_positions";

#[derive(Clone)]
pub struct Projection {
    tree: Tree,
    name: String,
    id: u64,
    last_seen_event_id: Option<u64>,
    last_relevant_event_id: Option<u64>,
}

impl Projection {
    pub(crate) fn new(db: &Db, name: String) -> Result<Self> {
        let tree = db.open_tree(PROJECTION_POSITIONS_TREE)?;

        let res = ProjectionPositionIter::new(tree.iter())
            .find_map(|res| {
                res.and_then(|raw_projection_data| {
                    let ProjectionData {
                        name: this_name,
                        last_seen_event_id,
                        last_relevant_event_id,
                    } = raw_projection_data.projection_data()?;
                    if name == this_name {
                        Ok(Some((
                            raw_projection_data.id()?,
                            last_seen_event_id,
                            last_relevant_event_id,
                        )))
                    } else {
                        Ok(None)
                    }
                })
                .transpose()
            })
            .transpose()?;

        let (id, last_seen_event_id, last_relevant_event_id) = match res {
            Some((id, last_seen_event_id, last_relevant_event_id)) => {
                (id, Some(last_seen_event_id), last_relevant_event_id)
            }
            None => (db.generate_id()?, None, None),
        };

        Ok(Projection {
            tree,
            name,
            id,
            last_seen_event_id,
            last_relevant_event_id,
        })
    }

    pub fn last_seen_event_id(&self) -> Option<u64> {
        self.last_seen_event_id
    }

    pub fn last_relevant_event_id(&self) -> Option<u64> {
        self.last_relevant_event_id
    }

    pub fn acknowledge_event(&mut self, position: u64, is_relevant: bool) -> Result<()> {
        let new_last_seen_event_id = position;
        let new_last_relevant_event_id = if is_relevant {
            Some(position)
        } else {
            self.last_relevant_event_id
        };

        let key = self.id.to_be_bytes();
        let value = bincode::serialize(&ProjectionData {
            name: &self.name,
            last_seen_event_id: new_last_seen_event_id,
            last_relevant_event_id: new_last_relevant_event_id,
        })
        .map_err(Error::SerializeProjection)?;
        self.tree.insert(key, value)?;
        self.last_seen_event_id = Some(new_last_seen_event_id);
        self.last_relevant_event_id = new_last_relevant_event_id;

        Ok(())
    }

    pub fn reset_position(&mut self) -> Result<()> {
        self.tree.remove(self.id.to_be_bytes())?;
        self.last_seen_event_id = None;
        self.last_relevant_event_id = None;

        Ok(())
    }
}

impl ops::Deref for Projection {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ProjectionData<'a> {
    name: &'a str,
    last_seen_event_id: u64,
    last_relevant_event_id: Option<u64>,
}

#[derive(Clone)]
struct RawProjectionData {
    key: IVec,
    value: IVec,
}

impl RawProjectionData {
    fn new(key: IVec, value: IVec) -> Self {
        RawProjectionData { key, value }
    }

    fn id(&self) -> Result<u64> {
        let slice = self
            .key
            .as_ref()
            .try_into()
            .map_err(|_| Error::InvalidU64Id)?;
        Ok(u64::from_be_bytes(slice))
    }

    fn projection_data<'a>(&'a self) -> Result<ProjectionData<'a>> {
        bincode::deserialize(&self.value).map_err(Error::DeserializeProjection)
    }
}

struct ProjectionPositionIter {
    inner: sled::Iter,
}

impl ProjectionPositionIter {
    fn new(inner: sled::Iter) -> Self {
        ProjectionPositionIter { inner }
    }
}

impl Iterator for ProjectionPositionIter {
    type Item = Result<RawProjectionData>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map_err(Error::from)
                .map(|(k, v)| RawProjectionData::new(k, v))
        })
    }
}
