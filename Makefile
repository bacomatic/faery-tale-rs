.PHONY: plan-check docs-check rag-demo rag-demo-inc rag-index rag-index-inc sync-issues agent-bootstrap

Q ?= where is page flip handled

plan-check:
	bash scripts/plan_sync_check.sh

docs-check:
	bash scripts/check_docs_links.sh

rag-demo:
	bash scripts/rag_demo.sh "$(Q)"

rag-demo-inc:
	INDEX_INCREMENTAL=true INDEX_RESET=false bash scripts/rag_demo.sh "$(Q)"

rag-index: ## Rebuild RAG index from scratch
	bash scripts/rag_index.sh

rag-index-inc: ## Incrementally update RAG index (no reset)
	INDEX_INCREMENTAL=true INDEX_RESET=false bash scripts/rag_index.sh

sync-issues:
	bash scripts/sync_plan_from_github.sh

agent-bootstrap:
	bash scripts/agent_bootstrap.sh
