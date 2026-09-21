import { describe, expect, it } from 'vitest'
import { crossedOccurrence, formatCountdown, nextOccurrence, nextReminder } from './workReminders'
import type { WorkReminderSettings, WorkSettings } from '../types'

const daily = (time: string): WorkReminderSettings => ({ enabled: true, time, repeat: 'daily', date: null, completed: false })

describe('work reminders', () => {
  it('finds the next daily occurrence and rolls over after the time', () => {
    const now = new Date(2026, 8, 21, 18, 1)
    expect(nextOccurrence(daily('18:00'), now)?.getDate()).toBe(22)
  })

  it('does not schedule an expired one-time reminder', () => {
    const reminder: WorkReminderSettings = { enabled: true, time: '09:00', repeat: 'once', date: '2026-09-21', completed: false }
    expect(nextOccurrence(reminder, new Date(2026, 8, 21, 10, 0))).toBeNull()
  })

  it('chooses the nearest enabled reminder', () => {
    const settings: WorkSettings = { lunch: daily('12:00'), offWork: daily('18:00') }
    const result = nextReminder(settings, new Date(2026, 8, 21, 10, 0))
    expect(result?.kind).toBe('lunch')
    expect(formatCountdown(result!.at, new Date(2026, 8, 21, 10, 0))).toBe('02:00:00')
  })

  it('detects a reminder crossed between ticks', () => {
    expect(crossedOccurrence(daily('12:00'), new Date(2026, 8, 21, 11, 59, 59), new Date(2026, 8, 21, 12, 0, 0))).toBe(true)
  })
})
