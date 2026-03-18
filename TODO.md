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


- generate prompt for external llm system to assess our financial situation
    - high level
        - average spending per year
        - income per year
        - balance sheet per month in a year
    - then categories per month
    - extra transactions
    - system prompt - describe the data, add ability to assess financial situation


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