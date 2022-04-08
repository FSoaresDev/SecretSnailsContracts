# SecretSnailsContracts

## Description

- Minter Contract

  - Load token details (img + metadata) to mint (DONE)
  - SNIP20 to mint NFTs (DONE)
  - Multiple mints in a TX (DONE)
  - Mints are randomly choosed from the list of tokens (DONE)
  - A hidden parameter "Speed" between 1-100 is added to each token (DONE)
  - Revenue of mint split between addresses (TODO)
  - Whitelist Enabled (DONE)
  - Array of addresses that are authorized to update metadata of tokens after mint (DONE)
  - Endpoint to be called by those addresses that have authority to update metadata (TODO)

- NFT Contract
  - Add hidden parameters that cant even be seen by the owner, only a defined number of addresses (TODO)

## Secret Snails Nft

- Base Repo: git@github.com:baedrik/snip721-reference-impl.git
- Commit: 9014ec8beff0b48640be1cd662c05a3b2b9c5cc4

## Resources

- https://github.com/luminaryphi/secret-random-minting-snip721-impl
- https://github.com/baedrik/snip721-reference-impl
