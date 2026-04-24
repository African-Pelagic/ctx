<!-- ctx-guidance:start -->
## ctx

ctx guidance

- .context/ is managed by ctx.
- Do not directly edit .context documents except for recovery or repair work.
- Use ctx assemble before relevant work.
- Use ctx new, ctx append, and ctx supersede for context updates.
- Capture enough detail that a later agent can act without another interview.
- Prefer semantic coverage over verbosity.
- For each concern, try to record: the current claim, why it is true, what it depends on, what it excludes, and what would cause it to be superseded.
- Include decisions, assumptions, constraints, tradeoffs, and concrete examples when they remove ambiguity.
- Do not overfit the context to incidental implementation details that will churn quickly.
- Run ctx check after context changes.
- Respect .contextignore when deciding what belongs in managed context.
<!-- ctx-guidance:end -->
