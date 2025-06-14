
pub const COUNT_SP_WITHOUT_LABELS: u32 = 15;
pub const COUNT_SP_ALL_ISSUES: u32 = 25;

pub const  GET_ISSUES_QUERY: &str = r#"
    query GetIterationIssues($group: ID!, $iterId: ID!) {
        group(fullPath: $group) {
            projects(first: 100, includeSubgroups: true) {
                nodes {
                    webUrl
                    issues(state: opened, iterationId: [$iterId], first: 100) {
                        nodes {
                            iid
                            webUrl
                            weight
                            labels { nodes { title } }
                            assignees { nodes { username } }
                        }
                    }
                }
            }
        }
    }
"#;
