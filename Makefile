.PHONY: plan-check docs-check sync-issues

plan-check:
	bash scripts/plan_sync_check.sh

docs-check:
	bash scripts/check_docs_links.sh

sync-issues:
	bash scripts/sync_plan_from_github.sh
