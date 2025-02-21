use crate::token::store::TokenStore;

pub(crate) struct TokenController<T: TokenStore> {
    token_store: T,
}

impl<T: TokenStore> TokenController<T> {
    pub(crate) fn new(token_store: T) -> Self {
        Self { token_store }
    }

    // pub(crate) fn login(request: Request, response: Response)
}
