#requires -Version 7.0
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force dist | Out-Null
$version=(yq -r '.version' package.json).Trim()
git archive --format=zip --output="dist/Linen-$version-source.zip" HEAD
if($LASTEXITCODE){throw 'Source packaging failed'}
