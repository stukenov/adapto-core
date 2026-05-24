# Критический технический риск

Самые сложные части:

1. стабильные node/component IDs;
2. корректный DOM patching;
3. dependency tracking;
4. form state preservation;
5. reconnect/resume;
6. nested component lifecycle;
7. hot reload;
8. понятные compiler errors;
9. безопасность event/action protocol;
10. баланс между DSL и обычным Rust.

Главное архитектурное правило: DSL не должен становиться игрушечным языком. Он должен оставлять escape hatch в обычный Rust.
