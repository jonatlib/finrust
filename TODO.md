- Bugs:
    - Broken categories page
        - Stats not correct - it should be sum of all transactions per year.
        - Then would be nice to show also average per year
        - And also percentage of that category
            - Beware the categories are tree so it sums up in the tree
    - New categories per account


- be able to create instance of recurring transaction early and then don't count it in the future.
    - For eample pay car service two months early
    - write test for this

The goal here is to generate prompt for external system
to asses your situation.

For this you must return in prompt, all current accounts with current balances.
Add names goals, etc... data about the accounts. Account types.

Then per account show balances per month, one data point per month at the end of a month.
This way the LLM can asses your situatino per account.

Also show incomes per year.

- generate prompt for external llm system to assess our financial situation
    - high level
        - average spending per year
        - income per year
        - balance sheet per month in a year
    - then categories per month - just average soending or income
    - extra transactions - don't list it all, just extratct info per category.
    - system prompt - describe the data, add ability to assess financial situation
    - prompt:
        - you are a best financial advisor in the world
        - Asses current sitiation and where we are heading
        - Suggest also what would be a goal in 12months, 24, 36months
        - Also check if money are not scattered everywhere, check goals


- dual entry accounting
    - manual account state - difference from model should be propagated as transaction rather then snapshot (keep both)
    - add error account
        - balance sheet of that error account in time


- osvc fortecast income
    - instances to create specific income
    - MD rate - calendar
    - expected vacation
    - expected salaries
    - extra invoicing - multiple extras (+/-)
    - select year
    - beware invoicing is done after the month - last month is in the new year, last month of previosu month is in
      current year
    - vacation days per month


- LLM export still active recurring transactions in TOML for LLM
    - add export from banking
    - LLM again in toml format suggest changes based on real expences
    - the export from LLM will define fix schema to use -
    - then be able to import this suggested changes back
    - before importing them show a diff of current transaction and suggested change and be able to select which will be
      accepted.
    - then import and change
    - be mindful of account - only one account exported and one account compared from bank


- LLM export of recurring transactions
    - based on bank export
    - same be mindful of what account exported and compared


- suggest missing recurring transactions


- Implement MCP server
    - Expose APIs like
        - Categories and yearly spend there
        - Account statuses - now, at the end of the year
        - Average throug aout the year