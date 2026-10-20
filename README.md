# Klausel

Desktop builds of Klausel for macOS, Windows and Linux, on x86-64 and ARM64. Download page: https://bhenkemans.github.io/klausel/

The source lives in the private repository `BHenkemans/automating-reasoning`. Pushing a `v*` tag there runs its `publish.yml`, which starts `release.yml` here. That workflow checks out the tag, bundles all six targets and attaches them to a release with stable asset names (`Klausel-<os>-<arch>.<ext>`). The page in `site/` links to `releases/latest/download/…`, so it doesn't need redeploying for new releases.

## Setup

1. **Token so this repo can read the source.** Create a fine-grained personal access token for repository `BHenkemans/automating-reasoning` only, with permission *Contents: Read-only*. Store it in this repo → Settings → Secrets → Actions as `SOURCE_TOKEN`. An SSH deploy key doesn't work here: the Windows ARM runner leaves the key file readable by other users, and SSH then refuses to use it.
2. **Token so the source repo can start a release here.** Create a fine-grained personal access token for repository `BHenkemans/klausel` only, with permission *Actions: Read and write*. Store it in `automating-reasoning` → Settings → Secrets → Actions as `KLAUSEL_RELEASE_TOKEN`.
3. **Pages.** In this repo, go to Settings → Pages → Source and select *GitHub Actions*.
4. **Fork pull requests.** In this repo, go to Settings → Actions → General and require approval for workflows from all outside collaborators.

To rebuild a tag by hand: Actions → Release → Run workflow, then enter the tag.
