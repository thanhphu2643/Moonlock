// MoonlockKit — native Apple glue for "Moonlock".
//
// Writes deploy-freeze days and lunar reminders (rằm, mùng 1, giỗ…) into a
// dedicated calendar in Apple Calendar via EventKit, so the freeze shows up
// everywhere the user's iCloud calendar syncs (iPhone, Watch, coworkers' invites).
//
// Exposed to Rust through a plain C ABI (no Swift types cross the boundary):
//   int32_t moonlock_calendar_sync(const char *json);
//
// `json` is an array of {"date":"YYYY-MM-DD","title":"…","notes":"…","alarm_days":1|null}.
// Returns number of events created (>= 0), or:
//   -1 calendar access denied, -2 bad JSON, -3 could not get/create calendar, -4 save failed.

import EventKit
import Foundation
import CoreGraphics

private let calendarTitle = "Moonlock"

private struct Item: Decodable {
    let date: String
    let title: String
    let notes: String
    let alarm_days: Int?
}

private func requestAccess(_ store: EKEventStore) -> Bool {
    let done = DispatchSemaphore(value: 0)
    var granted = false
    let completion: (Bool, Error?) -> Void = { ok, _ in
        granted = ok
        done.signal()
    }
    if #available(macOS 14.0, iOS 17.0, *) {
        store.requestFullAccessToEvents(completion: completion)
    } else {
        store.requestAccess(to: .event, completion: completion)
    }
    done.wait()
    return granted
}

private func ownCalendar(_ store: EKEventStore) -> EKCalendar? {
    if let existing = store.calendars(for: .event).first(where: { $0.title == calendarTitle }) {
        return existing
    }
    let cal = EKCalendar(for: .event, eventStore: store)
    cal.title = calendarTitle
    cal.cgColor = CGColor(red: 0.86, green: 0.18, blue: 0.22, alpha: 1)
    // Prefer the source of the default calendar (usually iCloud) so it syncs to the phone.
    guard let source = store.defaultCalendarForNewEvents?.source
        ?? store.sources.first(where: { $0.sourceType == .local }) else { return nil }
    cal.source = source
    do {
        try store.saveCalendar(cal, commit: true)
        return cal
    } catch {
        return nil
    }
}

private let dayFormatter: DateFormatter = {
    let f = DateFormatter()
    f.calendar = Calendar(identifier: .gregorian)
    f.locale = Locale(identifier: "en_US_POSIX")
    f.timeZone = TimeZone.current
    f.dateFormat = "yyyy-MM-dd"
    return f
}()

@_cdecl("moonlock_calendar_sync")
public func moonlockCalendarSync(_ json: UnsafePointer<CChar>) -> Int32 {
    guard let data = String(cString: json).data(using: .utf8),
          let items = try? JSONDecoder().decode([Item].self, from: data) else { return -2 }

    let store = EKEventStore()
    guard requestAccess(store) else { return -1 }
    guard let calendar = ownCalendar(store) else { return -3 }

    var created: Int32 = 0
    for item in items {
        guard let start = dayFormatter.date(from: item.date),
              let end = Calendar.current.date(byAdding: .day, value: 1, to: start) else { continue }

        // Idempotent: skip if an event with the same title already exists that day.
        let predicate = store.predicateForEvents(withStart: start, end: end, calendars: [calendar])
        if store.events(matching: predicate).contains(where: { $0.title == item.title }) { continue }

        let event = EKEvent(eventStore: store)
        event.calendar = calendar
        event.title = item.title
        event.notes = item.notes
        event.isAllDay = true
        event.startDate = start
        event.endDate = end
        if let days = item.alarm_days {
            // Remind at 09:00 the day(s) before: -days*24h + 9h.
            event.addAlarm(EKAlarm(relativeOffset: TimeInterval(-days * 86_400 + 9 * 3_600)))
            event.availability = .free
        } else {
            event.availability = .busy
        }
        do {
            try store.save(event, span: .thisEvent, commit: false)
            created += 1
        } catch {
            continue
        }
    }
    do { try store.commit() } catch { return -4 }
    return created
}
