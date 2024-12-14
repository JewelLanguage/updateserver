Auto Update Support
====

This is the first Hypertrail feature! A necessary one at that.
Hypertrail is a clean fork of chromium so just like its parent project, it does not come with autoupdate functionality. This means any changes or new features made to chromium or hypertrail itself will need the user to be able to pull from the main or change branches of each project individually, fix all conflicts, build and package the build for the target platform.
While this is a fun experience for people working on the project, i.e me, it's not a great experience for the general user and would definitely lead to some compatibility issues since there maybe changes merged into chromium that I don't want in Hypertrail. 

This warrants the need for Auto-Update support. Only after this feature is done will Hypertrail release to windows and macos. In other words, only linux users will be able to use the pre-autoupdate builds of Hypertrail and will have to download the application packages from the release page on github.

## Functionaly Requirements
There are two components required for autoupdates to work:
### Hypertrail Browser Component
- Ability to look up version updates
- Detect version updates
- Download updates in the background
- Schedule update time to happen in the background
- Resume execution after update with little to no user interference (No manual restart of the application or OS for the update to apply)
- Notify user of detected update
- Notify user of downloaded update
- Notify user of applied update
- List all new functionality/updates from merge
- If user is more than n-1 behind, offer version selection and feature list for version

### Hypertrail Update Server
- Store version updates upto n-5, to avoid permanennt breaking changes and allow quick mitigation
    - Release date
    - Release Notes
    - Version Number
    - OS Specific Binaries/Packages
- Handle update info request
- Handle update download request for latest and version number
- Track version number usage statistics

## Implementation
### Browser Component
Most implementation details for the browser have already been dealt with, we will be using reusing as much existing code as possible and only add our code when necessary. 

The release schedule has not been decided, due to version numbering, merge scheduling and coding schedules not being worked out. So until further notice, the initial frequecy of update version/download requests will be once a week. This is to account for urgernt merging and releasing

### Options

1. We could use the PushNotification Library to make and receive calls to the server:
2. We could use a scheduled background task that checks every once while(interval could be specified later) to handle updates.

### Solution: Hybrid

1. Use a background task for regular scheduled updates.
2. Use push notification service for urgent/realtime updates



### Server Component:
#### Packaging Script
- This script will pull all changes from chromium's main branch, execute a test pass and send me an email. I would like for the script to continously run a diff check and only merge when changes are made. 
This is the merge policy inorder of priority:
    -   Security Updates 
    -   Bug Fixes

The script will need to parse all commit messages in the merge and pull appropriate commits based on the policy above.
#### Note: Will need to figure out how to determine if a merge is urgent

If conflicts are detected merging stops and I get an email, with the error messages. In this case I'll handle the conflict and finish the merge manually.

The script will then have logic to build the entire codebase, and generate the appropriate package bundles for each os/platform and then save them in the appropriate directory in the server to be downloaded/queried by the client.

```yml
Request: [
    "GET /favicon.ico HTTP/1.1",
    "Host: 127.0.0.1:7778",
    "Connection: keep-alive",
    "sec-ch-ua-platform: \"Linux\"",
    "Accept-Language: en-US,en;q=0.9",
    "sec-ch-ua: \"Chromium\";v=\"133\", \"Not(A:Brand\";v=\"99\"",
    "User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "sec-ch-ua-mobile: ?0",
    "Accept: image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
    "Sec-Fetch-Site: same-origin",
    "Sec-Fetch-Mode: no-cors",
    "Sec-Fetch-Dest: image",
    "Referer: http://127.0.0.1:7778/",
    "Accept-Encoding: gzip, deflate, br, zstd",
]


```