Checkout attached transaction.rs file, which is able to create dataframes or rows or transactions from model and also
from recurring transactions.

Update all(!) computer to take advantage of this, as of now all the computers needs to compute the transactions when
they are and etc... this way, we can just work with something like `dyn TransactionGenerator` or other traits, and this
way abstrack out we have different kind of transactions and just try to generate the series from this.

See also that there is a transaction file in model workspace which implements ability to convert a single transaction
model into a multiple simple transactions struct and then in compute iterator of these transactions into dataframe.

This way you can completely abstract out that we have different kind of models, we can just load them up, convert to
something like `Vec<Box<dyn TransactionGenerator>>` and then work with these and generate series and just concate
generated dataframes.

Using this method
`fn generate_transactions(&self, start: NaiveDate, end: NaiveDate) -> impl Iterator<Item = Transaction>;`

Keep separated that changes to model should be in model workspace and anything working with polars should be in in
compute workspace.

Also try your best not to add specific transaction handling, as my goal is to be able to add new transaction model in
the future and just implement these traits and it should magicly work.

Also don't forget this is new Rust where you don't need `mod.rs` files! it is now that file is named same as the
directory.

Also this is several times we are trying this, last time you've implemented some generic generator in the compute, but
some computers need specific handling of a transactions like unpaid etc... so you can keep the specific handling of
specific transaction kinds, just use the new functionality with simple transactions.

And lastly check tests in compute workspace if they are not falling.
