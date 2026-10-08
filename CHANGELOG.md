# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Initial release: `create_campaign`, `contribute`, `withdraw`, `refund`, `cancel`, `bump`,
  `bump_contribution`, and read-only views.
- Events: `campaign_created`, `contributed`, `withdrawn`, `refunded`, `cancelled`.
- Balance-delta guard against fee-on-transfer and rebasing tokens.
- 24 tests including a model-based test and mock-token failure tests.
