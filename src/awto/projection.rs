pub trait DomainProjection {}

pub trait DomainProjectionList {
    fn exec(self);
}

impl DomainProjectionList for () {
    fn exec(self) {}
}

impl<A> DomainProjectionList for (A,)
where
    A: DomainProjection + 'static,
{
    fn exec(self) {
        // do stuff on self.0
    }
}

impl<A, B> DomainProjectionList for (A, B)
where
    A: DomainProjection + 'static,
    B: DomainProjection + 'static,
{
    fn exec(self) {
        // do stuff on self.0, self.1
    }
}

impl<A, B, C> DomainProjectionList for (A, B, C)
where
    A: DomainProjection + 'static,
    B: DomainProjection + 'static,
    C: DomainProjection + 'static,
{
    fn exec(self) {
        // do stuff on self.0, self.1, self.2
    }
}
