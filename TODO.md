- show per account current state, and state at the end of current month.


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
