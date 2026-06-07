import type { ToolPathVerificationRecord } from '../types/index.ts'

export function buildToolPathVerificationSummary(records: ToolPathVerificationRecord[]) {
  let verified = 0
  let unverified = 0
  let writeSupported = 0
  let writeUnsupported = 0

  for (const record of records) {
    if (record.platform === 'macos' && record.candidate_verified) {
      verified++
    } else {
      unverified++
    }
    
    if (record.write_supported) {
      writeSupported++
    } else {
      writeUnsupported++
    }
  }

  const labels: string[] = []
  if (unverified > 0) labels.push(`${unverified} 个待核验候选路径`)
  if (verified > 0) labels.push(`${verified} 个已核验路径`)
  if (writeUnsupported > 0) labels.push(`${writeUnsupported} 个写入 unsupported`)
  if (writeSupported > 0) labels.push(`${writeSupported} 个可写路径`)

  return {
    total: records.length,
    verified,
    unverified,
    writeSupported,
    writeUnsupported,
    labels
  }
}
