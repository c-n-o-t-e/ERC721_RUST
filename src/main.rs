/*
   The ink! attribute is used to define a smart contract.
   similar to mapping sol but this comes rust saftey and type system
   scale is a serialization library for encoding and decoding data in SCALE format
   (scale means Simple Concatenated Aggregate Little-Endian)
*/
#[ink::contract]
mod erc721 {
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    pub type TokenId = u32;

    /*
        contract struct
        ink::storage is used to define the storage of the smart contract.
        derive(Default) is used to define the default value of the storage.
        debug
    */
    #[ink::storage]
    #[derive(Default)]
    pub struct Erc721 {
        token_owner: Mapping<TokenId, AccountId>,
        token_approvals: Mapping<TokenId, AccountId>,
        owned_tokens_count: Mapping<AccountId, u32>,
        operator_approvals: Mapping<AccountId, AccountId, ()>,
    }

    /*
        error enum
        debug and derive(Encode, Decode) is used to define the serialization and deserialization of the error.
        derive(PartialEq, Eq, Clone) is used to define the comparison of the error.
        derive(Copy) is used to define the copy of the error.
        derive(scale_info::TypeInfo) is used to define the type information of the error.
        cfg_attr is used to define the configuration attribute of the error.
        TypeInfo used to provide metadata about the type.
    */
    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        NotApproved,
        TokenExists,
        TokenNotFound,
        CannotInsert,
        NotAllowed,
        CannotFetchValue,
    }

    #[ink{event}]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        id: TokenId,
    }

    #[ink{event}]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        id: TokenId,
    }

    #[ink{event}]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    #[ink{constructor}]
    pub fn new() -> Self {
        Default::default()
    }

    // #[ink(message)] is used to define the message of the smart contract.
    #[ink(message)]
    pub fn balance_of(&self, owner: AccountId) -> u32 {
        self.balance_of_or_zero(&owner)
    }

    #[ink(message)]
    pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
        self.token_owner.get(id)
    }

    /// Transfers the token from the caller to the given destination.
    #[ink(message)]
    pub fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        self.transfer_token_from(&caller, &destination, id)?;
        Ok(())
    }

    /// Transfer approved or owned token.
    #[ink(message)]
    pub fn transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        id: TokenId,
    ) -> Result<(), Error> {
        self.transfer_token_from(&from, &to, id)?;
        Ok(())
    }

    // Returns the total number of tokens from an account.
    fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
        self.owned_tokens_count.get(of).unwrap_or(0)
    }

    /// Transfers token `id` `from` the sender to the `to` `AccountId`.
    fn transfer_token_from(
        &mut self,
        from: &AccountId,
        to: &AccountId,
        id: TokenId,
    ) -> Result<(), Error> {
        let caller = self.env().caller();
        if !self.exists(id) {
            return Err(Error::TokenNotFound);
        };

        // if !self.owner_of(id).map_or(false, |owner| owner == *from) {
        //     return Err(Error::NotOwner)
        // };

        if !self.approved_or_owner(Some(caller), id) {
            return Err(Error::NotApproved);
        };

        self.clear_approval(id);
        self.remove_token_from(from, id)?;
        self.add_token_to(to, id)?;

        self.env().emit_event(Transfer {
            from: Some(*from),
            to: Some(*to),
            id,
        });

        Ok(())
    }

    /// Returns true if token `id` exists or false if it does not.
    fn exists(&self, id: TokenId) -> bool {
        self.token_owner.contains(id)
    }

    /// Returns true if the `AccountId` `from` is the owner of token `id`
    /// or it has been approved on behalf of the token `id` owner.
    fn approved_or_owner(&self, from: Option<AccountId>, id: TokenId) -> bool {
        let owner = self.owner_of(id);
        // same as if(from !=address(0) && (from == owner || from == self.token_approvals.get(id) || self.approved_for_all(owner, from)))
        from != Some(AccountId::from([0x0; 32]))
            && (from == owner
                || from == self.token_approvals.get(id)
                || self.approved_for_all(
                    owner.expect("Error with AccountId"), // expect is used to handle the error if address is not found
                    from.expect("Error with AccountId"),
                ))
    }

    #[ink(message)]
    pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
        self.approve_for(&to, id)?;
        Ok(())
    }

    /// Approves or disapproves the operator for all tokens of the caller.
    #[ink(message)]
    pub fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
        self.approve_for_all(to, approved)?;
        Ok(())
    }

    /// Approve the passed `AccountId` to transfer the specified token on behalf of
    /// the message's sender.
    fn approve_for(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        let owner = self.owner_of(id);
        if !(owner == Some(caller)
            || self.approved_for_all(owner.expect("Error with AccountId"), caller))
        {
            return Err(Error::NotAllowed);
        };

        if *to == AccountId::from([0x0; 32]) {
            return Err(Error::NotAllowed);
        };

        if self.token_approvals.contains(id) {
            return Err(Error::CannotInsert);
        } else {
            self.token_approvals.insert(id, to);
        }

        self.env().emit_event(Approval {
            from: caller,
            to: *to,
            id,
        });

        Ok(())
    }

    /// Approves or disapproves the operator to transfer all tokens of the caller.
    fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
        let caller = self.env().caller();
        if to == caller {
            return Err(Error::NotAllowed);
        }
        self.env().emit_event(ApprovalForAll {
            owner: caller,
            operator: to,
            approved,
        });

        if approved {
            self.operator_approvals.insert((&caller, &to), &());
        } else {
            self.operator_approvals.remove((&caller, &to));
        }

        Ok(())
    }

    /// Creates a new token.
    #[ink(message)]
    pub fn mint(&mut self, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        self.add_token_to(&caller, id)?;
        self.env().emit_event(Transfer {
            from: Some(AccountId::from([0x0; 32])),
            to: Some(caller),
            id,
        });
        Ok(())
    }

    /// Deletes an existing token. Only the owner can burn the token.
    #[ink(message)]
    pub fn burn(&mut self, id: TokenId) -> Result<(), Error> {
        let caller = self.env().caller();
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        let owner = token_owner.get(id).ok_or(Error::TokenNotFound)?;
        if owner != caller {
            return Err(Error::NotOwner);
        };

        let count = owned_tokens_count
            .get(caller)
            .map(|c| c - 1)
            .ok_or(Error::CannotFetchValue)?;

        owned_tokens_count.insert(caller, &count);
        token_owner.remove(id);

        self.env().emit_event(Transfer {
            from: Some(caller),
            to: Some(AccountId::from([0x0; 32])),
            id,
        });

        Ok(())
    }

    /// Adds the token `id` to the `to` AccountID.
    fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        if token_owner.contains(id) {
            return Err(Error::TokenExists);
        }

        if *to == AccountId::from([0x0; 32]) {
            return Err(Error::NotAllowed);
        };

        let count = owned_tokens_count.get(to).map(|c| c + 1).unwrap_or(1);

        owned_tokens_count.insert(to, &count);
        token_owner.insert(id, to);

        Ok(())
    }

    /// Removes token `id` from the owner.
    fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
        let Self {
            token_owner,
            owned_tokens_count,
            ..
        } = self;

        if !token_owner.contains(id) {
            return Err(Error::TokenNotFound);
        }

        let count = owned_tokens_count
            .get(from)
            .map(|c| c - 1)
            .ok_or(Error::CannotFetchValue)?;
        owned_tokens_count.insert(from, &count);
        token_owner.remove(id);

        Ok(())
    }

    /// Removes existing approval from token `id`.
    fn clear_approval(&mut self, id: TokenId) {
        self.token_approvals.remove(id);
    }
}
