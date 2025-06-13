A bot that calculates the number of story points for issues without the labels `status::to-review` and `status::to-test`, where the threshold does not exceed 15, and for all issues where the threshold does not exceed 25.

In both cases, issues are moved to the next iteration if they have labels `priority::Minor` or `priority::Trivial` and do not have any labels starting with `customer::*` or `release::*`.

The bot processes issues either in a group or in a separate project, and retrieves the list of assignees from those issues.

It separately calculates the number of story points for:

Issues with labels `status::to-review` and `status::to-test`.

All issues (regardless of status).

Then, for each of the above two cases, it checks the following condition:

Issues with `priority::Minor` or `priority::Trivial` labels, and no `customer::*` or `release::*` labels.

Bot arguments:
- --host – GitLab URL, for example, https://gitlab.com
- --token – Personal GitLab access token
- --group-name – Name of the GitLab group (optional)
- --assignees – List of users for targeted processing
