orgs:
    - have `user`
    - have `teams`
    - have `projects`
    - can `initiate` or `confirm` cross-org relations
teams:
    - work `projects`
    - manage/partner/visit `teams`
users:
    - belong to `orgs`
    - are members of `teams`
    - administer `orgs`/`teams`/`projects`
    - participate in `projects` (must be members/guests of teams)
projs
