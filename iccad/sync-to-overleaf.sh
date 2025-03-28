#!/bin/bash -e
set -eo pipefail
ROOT=$(basename $(pwd))
if [ $ROOT != "iccad" ]; then
    echo "Please run this script from the iccad directory"
else

    source token.sh
    if [ -z $OTOK ]; then
        echo "Please provide a token"
    else

        rm -rf overleaf-project
        mkdir overleaf-project
        git clone https://git:$OTOK@git.overleaf.com/67e6e83f7fde409aee480653 overleaf-project
        git ls-files > files.txt
        cat files.txt | xargs -I '{}' cp --parents '{}' overleaf-project
        cd overleaf-project
        cat ../files.txt | xargs git add
        git commit -m "sync with GitHub"
        git push https://git:$OTOK@git.overleaf.com/67e6e83f7fde409aee480653 --force
        cd ..
        rm -rf overleaf-project files.txt
        echo "Overleaf project updated successfully."

    fi

fi
set +eo pipefail
