import { Info } from 'lucide-react'

export function UnitInfoIcon() {
  const explanation = 'Token 数量级缩写说明：\nK = 千 (Thousand)\nM = 百万 (Million)\nB = 十亿 (Billion)\n\n你可以将鼠标悬浮在具体的数值上查看精确数字。'
  
  return (
    <span
      className="inline-flex items-center justify-center ml-1 text-muted hover:text-brand-blue cursor-pointer transition-colors"
      title={explanation}
      onClick={(e) => {
        e.stopPropagation() // Prevent sorting when clicking in table header
        alert(explanation)
      }}
    >
      <Info size={14} />
    </span>
  )
}
