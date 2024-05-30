#[cfg(test)]
mod sources;

#[cfg(test)]
mod tests {
    struct TestContext {
        sess: Session<MinimalRuntime>,
       
        alice: AccountId32,
        bob: AccountId32,
        charlie: AccountId32,
        dave: AccountId32,
        ed: AccountId32,
    }
    fn setup() -> Result<TestContext, Box<dyn Error>> {
        let bob = AccountId32::new([1u8; 32]);
        let alice = AccountId32::new([2u8; 32]);
        let charlie = AccountId32::new([3u8; 32]);
        let dave = AccountId32::new([4u8; 32]);
        let ed = AccountId32::new([5u8; 32]);

        let mut sess: Session<MinimalRuntime> = Session::<MinimalRuntime>::new().unwrap();

        //sess.upload(helpers::bytes_registry()).expect("Session should upload registry bytes");
        //sess.upload(helpers::bytes_share_token()).expect("Session should upload token bytes");
        Ok(TestContext {
            sess,            
            alice,
            bob,
            charlie,
            dave,
            ed,
        })
    }
}