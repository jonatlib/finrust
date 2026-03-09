Fin avg expense per month and tumbling and chart and same for income

- Bugs:
    - Fix tests
    - Same color schema for accounts
        - Add account color as an atribute (migrations etc)
        - Make it that importer will assign colors - by sequence the accounts are created - so it is stable
        - Update all charts which show multiple accounts to follow this color scheme
        - Boubles on overview should follow again this color scheme - but also to show using some badge the account type
          color
    - Frontend not whole decimal places but one zero decimal places is enough
        - Mainly on the accounts page and account detail page
    - Transactions page (recurring and extra)
        - When using filters the pagination don't take them into account leading to broken pagination
    - Recent activities on dashboard are not sorted - showing old transactions.
    - Broken categories page
        - Stats not correct - it should be sum of all transactions per year.
        - Then would be nice to show also average per year
        - And also percentage of that category
            - Beware the categories are tree so it sums up in the tree
    - New categories per account
    - The stats on the account detail page don't match with data in balance - i believe the balance chart.
    - Frontend shows $ sign. Instead of curency based on account
        - This might lead to multicurrency - don't do it now. Just use currency by first account


- show per account current state, and state at the end of current month.


- show tumbling minimum per account to be able to see if the account state is actually growing or not
    - show this in account detail as new charts
        - one chart on the historical balances
        - and one for forecast.
    - Add also stats about this on the all accounts page - to be able to assess how the balance on tha account builds up


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


- multiple acconut types
    - some preparation done
    - debt, savings, equity, net worth including house etc...


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