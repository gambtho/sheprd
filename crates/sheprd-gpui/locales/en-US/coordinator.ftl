coordinator-missing = no coordinator
coordinator-needs-you = needs you
coordinator-ready = ready
coordinator-summary =
    { $lanes ->
        [one] 1 lane
       *[other] { $lanes } lanes
    }{ $needs ->
        [0] { "" }
        [one] { " · " }1 needs you
       *[other] { " · " }{ $needs } need you
    }
coordinator-stale = ◷ project info from { $hours } h ago
coordinator-updated-now = updated just now
coordinator-updated-minutes = updated { $minutes }m ago
coordinator-updated-hours = updated { $hours }h ago
