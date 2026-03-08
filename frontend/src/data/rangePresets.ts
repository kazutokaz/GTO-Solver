// Common preranking ranges for HU and 6-max cash
// Based on typical GTO opening ranges

export interface RangePreset {
  name: string;
  range: string;
}

export const PRESETS: Record<string, RangePreset[]> = {
  'RFI (6-max)': [
    {
      name: 'UTG',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,AKs,AQs,AJs,ATs,A5s,A4s,KQs,KJs,KTs,QJs,QTs,JTs,T9s,98s,87s,76s,65s,AKo,AQo',
    },
    {
      name: 'MP',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,66,AKs,AQs,AJs,ATs,A9s,A5s,A4s,A3s,KQs,KJs,KTs,K9s,QJs,QTs,Q9s,JTs,J9s,T9s,98s,87s,76s,65s,54s,AKo,AQo,AJo',
    },
    {
      name: 'CO',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,66,55,44,AKs,AQs,AJs,ATs,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KQs,KJs,KTs,K9s,K8s,QJs,QTs,Q9s,Q8s,JTs,J9s,J8s,T9s,T8s,98s,97s,87s,86s,76s,75s,65s,64s,54s,53s,43s,AKo,AQo,AJo,ATo,KQo,KJo,QJo',
    },
    {
      name: 'BTN',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,66,55,44,33,22,AKs,AQs,AJs,ATs,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KQs,KJs,KTs,K9s,K8s,K7s,K6s,K5s,K4s,QJs,QTs,Q9s,Q8s,Q7s,Q6s,JTs,J9s,J8s,J7s,T9s,T8s,T7s,98s,97s,96s,87s,86s,76s,75s,65s,64s,54s,53s,43s,AKo,AQo,AJo,ATo,A9o,A8o,A7o,A6o,A5o,KQo,KJo,KTo,K9o,QJo,QTo,Q9o,JTo,J9o,T9o,98o',
    },
    {
      name: 'SB',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,66,55,44,33,22,AKs,AQs,AJs,ATs,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KQs,KJs,KTs,K9s,K8s,K7s,K6s,K5s,K4s,K3s,K2s,QJs,QTs,Q9s,Q8s,Q7s,Q6s,Q5s,Q4s,Q3s,Q2s,JTs,J9s,J8s,J7s,J6s,J5s,T9s,T8s,T7s,T6s,98s,97s,96s,87s,86s,85s,76s,75s,74s,65s,64s,63s,54s,53s,43s,AKo,AQo,AJo,ATo,A9o,A8o,A7o,A6o,A5o,A4o,A3o,A2o,KQo,KJo,KTo,K9o,K8o,K7o,QJo,QTo,Q9o,Q8o,JTo,J9o,J8o,T9o,T8o,98o,97o,87o,76o,65o',
    },
  ],
  '3-Bet': [
    {
      name: 'vs UTG',
      range: 'AA,KK,QQ,JJ,AKs,AQs,AJs,A5s,A4s,KQs,AKo',
    },
    {
      name: 'vs CO',
      range: 'AA,KK,QQ,JJ,TT,99,AKs,AQs,AJs,ATs,A5s,A4s,A3s,KQs,KJs,QJs,JTs,T9s,AKo,AQo,AJo',
    },
    {
      name: 'vs BTN',
      range: 'AA,KK,QQ,JJ,TT,99,88,77,AKs,AQs,AJs,ATs,A9s,A8s,A5s,A4s,A3s,A2s,KQs,KJs,KTs,QJs,QTs,JTs,T9s,98s,87s,76s,65s,AKo,AQo,AJo,ATo,KQo',
    },
  ],
  'Call': [
    {
      name: 'BB vs BTN',
      range: 'TT,99,88,77,66,55,44,33,22,AJs,ATs,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KQs,KJs,KTs,K9s,K8s,K7s,K6s,K5s,QJs,QTs,Q9s,Q8s,Q7s,Q6s,JTs,J9s,J8s,J7s,T9s,T8s,T7s,98s,97s,96s,87s,86s,76s,75s,65s,64s,54s,53s,43s,AQo,AJo,ATo,A9o,A8o,A7o,A6o,A5o,A4o,A3o,KQo,KJo,KTo,K9o,QJo,QTo,Q9o,JTo,J9o,T9o,98o,87o',
    },
    {
      name: 'BB vs CO',
      range: 'TT,99,88,77,66,55,44,33,22,ATs,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KJs,KTs,K9s,K8s,K7s,K6s,QJs,QTs,Q9s,Q8s,Q7s,JTs,J9s,J8s,J7s,T9s,T8s,T7s,98s,97s,96s,87s,86s,76s,75s,65s,64s,54s,53s,43s,AJo,ATo,A9o,A8o,A7o,KQo,KJo,KTo,K9o,QJo,QTo,Q9o,JTo,J9o,T9o',
    },
  ],
};
