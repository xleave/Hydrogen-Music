import { storeToRefs } from 'pinia'
import pinia from '../../store/pinia'
import { useOtherStore } from '../../store/otherStore'
import { usePlayerStore } from '../../store/playerStore'

export const otherStore = useOtherStore(pinia)
export const playerStore = usePlayerStore(pinia)
export const playerRefs = storeToRefs(playerStore)
