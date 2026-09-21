import type { WorkReminderSettings, WorkSettings } from '../types'

export type ReminderKind = 'lunch' | 'offWork'

export interface ReminderOccurrence {
  kind: ReminderKind
  at: Date
}

export function parseTime(time: string): { hours: number; minutes: number } | null {
  const match = /^(\d{2}):(\d{2})$/.exec(time)
  if (!match) return null
  const hours = Number(match[1])
  const minutes = Number(match[2])
  return hours < 24 && minutes < 60 ? { hours, minutes } : null
}

export function isValidReminder(reminder: WorkReminderSettings): boolean {
  if (!reminder.enabled || reminder.completed || !parseTime(reminder.time)) return false
  return reminder.repeat === 'daily' || (reminder.repeat === 'once' && Boolean(reminder.date))
}

function dateKey(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function occurrenceOnDate(reminder: WorkReminderSettings, date: Date): Date | null {
  if (!isValidReminder(reminder)) return null
  const day = reminder.repeat === 'once' ? reminder.date : dateKey(date)
  if (!day) return null
  const parsed = parseTime(reminder.time)
  if (!parsed) return null
  const [year, month, dayNumber] = day.split('-').map(Number)
  const result = new Date(year, month - 1, dayNumber, parsed.hours, parsed.minutes, 0, 0)
  return Number.isNaN(result.getTime()) ? null : result
}

export function nextOccurrence(reminder: WorkReminderSettings, now: Date): Date | null {
  if (!isValidReminder(reminder)) return null
  const today = occurrenceOnDate(reminder, now)
  if (reminder.repeat === 'once') return today && today > now ? today : null
  if (today && today > now) return today
  const tomorrow = new Date(now)
  tomorrow.setDate(tomorrow.getDate() + 1)
  return occurrenceOnDate(reminder, tomorrow)
}

export function nextReminder(settings: WorkSettings, now: Date): ReminderOccurrence | null {
  const candidates = (Object.entries(settings) as Array<[ReminderKind, WorkReminderSettings]>)
    .map(([kind, reminder]) => {
      const at = nextOccurrence(reminder, now)
      return at ? { kind, at } : null
    })
    .filter((item): item is ReminderOccurrence => item !== null)
    .sort((left, right) => left.at.getTime() - right.at.getTime())
  return candidates[0] ?? null
}

export function crossedOccurrence(reminder: WorkReminderSettings, previous: Date, now: Date): boolean {
  if (!isValidReminder(reminder) || now <= previous) return false
  const currentDay = occurrenceOnDate(reminder, now)
  if (currentDay && previous < currentDay && now >= currentDay) return true
  if (reminder.repeat === 'daily') {
    const previousDay = new Date(now)
    previousDay.setDate(previousDay.getDate() - 1)
    const yesterday = occurrenceOnDate(reminder, previousDay)
    return Boolean(yesterday && previous < yesterday && now >= yesterday && now.getDate() !== previous.getDate())
  }
  return false
}

export function formatCountdown(target: Date, now: Date): string {
  const seconds = Math.max(0, Math.floor((target.getTime() - now.getTime()) / 1000))
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const rest = seconds % 60
  return [hours, minutes, rest].map((value) => String(value).padStart(2, '0')).join(':')
}
