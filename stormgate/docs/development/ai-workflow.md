# Working with AI agents

Storm Gate is developed with help from AI coding agents. The rules:

- Tasks are small issues with acceptance criteria ([AGENTS.md](../../AGENTS.md)).
- Agents follow [CLAUDE.md](../../CLAUDE.md) and the area guides in `.ai/`.
- Loop: issue → analysis → small patch → build → automated test → test on a
  real Mac → logs → analysis → next patch.
- Never: an agent generates 20 000 lines → direct merge.
- Big decisions become ADRs so agents and new contributors do not reinvent
  the architecture.
