#!/bin/bash

export PATH="$PATH:/opt/homebrew/bin:/usr/local/bin:/etc/profiles/per-user/$USER/bin"

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"/..

TAG_PREFIX="builds/"

CURRENT_BUILD=$(git tag --points-at HEAD | tail -n 1)
LATEST_BUILD="$TAG_PREFIX$(git tag -l "builds/[0-9]*" | cut -d'/' -f 2 | sort -n | tail -n 1)"

CURRENT_BUILD_NUMBER=${CURRENT_BUILD#$TAG_PREFIX}
LATEST_BUILD_NUMBER=${LATEST_BUILD#$TAG_PREFIX}
if [[ -z $LATEST_BUILD_NUMBER ]]; then
    LATEST_BUILD_NUMBER="0"
fi

if [[ ! -z $LATEST_BUILD && $(git merge-base --is-ancestor $LATEST_BUILD HEAD) -ne 0 ]]; then
    echo "error: HEAD is not descended from build $LATEST_BUILD_NUMBER" >&2
    exit 1
fi

BUILD_NUMBER=$LATEST_BUILD_NUMBER

if [[ $# -gt 0 && "$1" == "increment" ]]; then
    NEW_BUILD_NUMBER=$((LATEST_BUILD_NUMBER + 1))
    NEW_TAG="$TAG_PREFIX$NEW_BUILD_NUMBER"
    BUILD_NUMBER=$NEW_BUILD_NUMBER

    git tag $NEW_TAG
    git push --quiet origin $NEW_TAG
    gh release create "$NEW_TAG" -t "Build $BUILD_NUMBER" --verify-tag --generate-notes >/dev/null
fi

if [[ -z $(grep $BUILD_NUMBER Apple/Configuration/Version.xcconfig 2>/dev/null) ]]; then
    echo "CURRENT_PROJECT_VERSION = $BUILD_NUMBER" > Apple/Configuration/Version.xcconfig
    git update-index --assume-unchanged Apple/Configuration/Version.xcconfig
fi

if [[ $# -gt 0 && "$1" == "status" ]]; then
    if [[ $CURRENT_BUILD_NUMBER -eq $LATEST_BUILD_NUMBER ]]; then
        echo "clean"
    else
        echo "dirty"
    fi
    exit 0
fi

echo $BUILD_NUMBER
