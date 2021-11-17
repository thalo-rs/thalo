use super::{aggregate::DomainAggregateList, projection::DomainProjectionList};

pub trait DomainService {
    type Aggregates: DomainAggregateList;
    type Projections: DomainProjectionList;
}
